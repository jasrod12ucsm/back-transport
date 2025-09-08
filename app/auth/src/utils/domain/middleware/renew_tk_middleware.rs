use ac_struct_back::utils::domain::errors::{BadRequestError, ErrorGenerate};
use ntex::{
    Middleware,
    http::{StatusCode, header},
    web,
};

pub struct RenewTokenMiddleware;

impl<S> Middleware<S> for RenewTokenMiddleware {
    type Service = RenewTokenMiddlewareService<S>;

    fn create(&self, service: S) -> Self::Service {
        RenewTokenMiddlewareService { service }
    }
}

pub struct RenewTokenMiddlewareService<S> {
    service: S,
}

impl<S, Err> ntex::Service<web::WebRequest<Err>> for RenewTokenMiddlewareService<S>
where
    S: ntex::Service<web::WebRequest<Err>, Response = web::WebResponse, Error = web::Error>,
    Err: web::ErrorRenderer,
{
    type Response = web::WebResponse;
    type Error = web::Error;

    ntex::forward_ready!(service);

    async fn call(
        &self,
        req: web::WebRequest<Err>,
        ctx: ntex::ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        //traer los headers
        let header = req.headers();

        if let Some(resp) = header
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .filter(|token| !token.is_empty())
            .map(|val| {
                let parts: Vec<&str> = val.split(' ').collect();
                let token = parts.get(1);
                if token.is_none() {
                    return Some(BadRequestError::render_by_status(
                        "Authorization Error",
                        "Credenciales no enviadas o inválidas",
                        StatusCode::UNAUTHORIZED,
                    ));
                }
                None
            })
            .unwrap_or_else(|| {
                Some(BadRequestError::render_by_status(
                    "Authorization Error",
                    "Credenciales no enviadas o inválidas",
                    StatusCode::UNAUTHORIZED,
                ))
            })
        {
            return Ok(req.into_response(resp));
        }
        let res = ctx.call(&self.service, req).await?;
        Ok(res)
    }
}
