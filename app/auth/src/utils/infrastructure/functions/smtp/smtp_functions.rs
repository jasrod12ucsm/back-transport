use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::{authentication::Credentials, response::Response},
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::CONFIG;

pub struct SmtpFunctions;

impl SmtpFunctions {
    pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<Response, String> {
        //let email
        //TODO ponerlo en el env
        let to_address = to
            .parse::<Address>()
            .map_err(|_| "error to parse address")?;
        let enterprise_email = CONFIG.SMTP_EMAIL.clone();
        let enterprise_password = CONFIG.SMTP_PASSWORD.clone();
        let address_data = enterprise_email.split("@").collect::<Vec<&str>>();
        let name = address_data.get(0).ok_or("error to get address name")?;
        let domain = address_data.get(1).ok_or("error to get address domain")?;
        let address = Address::new(*name, *domain).map_err(|_| "error to construct address")?;
        let email = Message::builder()
            .from(Mailbox::new(None, address))
            .to(Mailbox::new(None, to_address))
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(String::from(body))
            .map_err(|_| "error in message")?;
        let creds = Credentials::new((*name).to_string(), enterprise_password);
        // let mailer = SmtpTransport::relay("smtp.gmail.com")
        //     .unwrap()
        //     .port(465)
        //     .credentials(creds)
        //     .tls(Tls::Wrapper(
        //         TlsParameters::builder("smtp.gmail.com".to_string())
        //             .build()
        //             .map_err(|_| "error on tls")?,
        //     ))
        //     .build();

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&CONFIG.SMTP_SERVER)
            .map_err(|_| "error on starttls relay")?
            .credentials(creds)
            .build();

        match mailer.send(email).await {
            Ok(val) => return Ok(val),
            Err(err) => return Err(err.to_string()),
        }
    }
}
