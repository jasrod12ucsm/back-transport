import polars as pl
import numpy as np
import json
import datetime as datetime

def process_dataframe_from_csv_bytes(csv_bytes: bytes, columns: list[str] | None = None):
    """Lee un CSV desde bytes con configuración detallada y selecciona columnas."""
    print("Procesando DataFrame desde bytes CSV...")

    df = pl.read_csv(
        csv_bytes,                # <- directamente bytes
        has_header=True,          # primera fila son nombres de columnas
        columns=columns,          # selecciona solo las columnas que Rust manda
        separator=";",            # separador por defecto
        quote_char='"',           # campos entre comillas
        eol_char="\n",            # salto de línea estándar
        encoding="utf8",          # UTF-8
        ignore_errors=False,      # no ignorar errores
        try_parse_dates=True,     # intentar detectar fechas
        n_threads=None,           # usar todos los cores
        infer_schema=True,        # inferir tipos
        infer_schema_length=1000, # analizar primeras 1000 filas
        low_memory=False,         # más rápido, consume más RAM
        rechunk=True,             # optimizar chunks
        truncate_ragged_lines=True, # completar filas incompletas
        decimal_comma=False       # usar punto como decimal
    )

    # Metadata de utilidad

    return df

def compute_stats(df: pl.DataFrame, continuous_cols: list[str]) -> pl.DataFrame:
    if not continuous_cols:
        return pl.DataFrame()

    exprs = []
    for name in continuous_cols:
        print(name)
        col = pl.col(name).drop_nulls().drop_nans().cast(pl.Float64)
        exprs.extend([
            col.min().alias(f"{name}_min"),
            col.max().alias(f"{name}_max"),
            col.mean().alias(f"{name}_mean"),
            col.std().alias(f"{name}_std"),
            col.quantile(0.5, "linear").alias(f"{name}_median"),
            col.quantile(0.25, "linear").alias(f"{name}_q25"),
            col.quantile(0.75, "linear").alias(f"{name}_q75"),
        ])

    stats_df = df.lazy().select(exprs).collect(engine='gpu')
    return stats_df


def compute_features(df: pl.DataFrame, stats_df: pl.DataFrame, types: dict[str, str]) -> str:
    """Genera lista de features con stats y distribuciones de manera ultra rápida."""
    total_rows = df.height
    now_str = '2025-08-27T15:18:35.883Z'
    features = []

    # Precachear stats_df en dict para acceso rápido
    stats = {col: stats_df[col][0] for col in stats_df.columns}

    for name in df.columns:
        col = df[name]
        type_feature = types.get(name, "Categorical")
        misses_percent = int(round(col.null_count() / total_rows * 100)) if total_rows else 0

        feature = {
            "name": name,
            "count": total_rows,
            "misses_percent": misses_percent,
            "cardinality": col.n_unique(),
            "type_feature": type_feature,
            "created_at": now_str,
            "updated_at": now_str,
            "status": 0
        }

        if type_feature == "Continuous":
            # Usamos stats_df directamente
            min_v = float(stats.get(f"{name}_min", 0.0))
            max_v = float(stats.get(f"{name}_max", 0.0))
            mean_v = float(stats.get(f"{name}_mean", 0.0))
            std_v = float(stats.get(f"{name}_std", 0.0))
            median_v = float(stats.get(f"{name}_median", 0.0))
            q25_v = float(stats.get(f"{name}_q25", 0.0))
            q75_v = float(stats.get(f"{name}_q75", 0.0))

            feature.update({
                "min": str(min_v),
                "max": str(max_v),
                "mean": str(mean_v),
                "standard_deviation": str(std_v),
                "median": str(median_v),
                "per_quartil": str(q25_v),
                "tertile": str(q75_v)
            })

            # Freedman-Diaconis bin width vectorizado
            iqr = q75_v - q25_v
            series_non_null = col.drop_nulls().cast(pl.Float64)
            n = series_non_null.len()
            bin_width = 2 * iqr / (n ** (1/3)) if n > 1 else 1.0
            n_bins = int(np.ceil((max_v - min_v) / bin_width)) if bin_width > 0 else max(1, int(np.log2(n) + 1))
            width = (max_v - min_v) / n_bins if n_bins > 0 else 1.0

            if n_bins > 0 and n > 0:
                # Vectorized bin assignment
                idxs = ((series_non_null - min_v) / width).floor().clip(0, n_bins - 1).cast(pl.Int64)
                counts = idxs.value_counts().sort("values")  # values=bin index
                counts_array = np.zeros(n_bins, dtype=int)
                counts_array[counts["values"].to_numpy()] = counts["counts"].to_numpy()
            else:
                counts_array = np.zeros(n_bins, dtype=int)

            feature["distribution_bins"] = [str(min_v + i * width) for i in range(n_bins)]
            feature["distribution_intervals"] = [[float(min_v + i * width), float(min_v + (i + 1) * width)] for i in range(n_bins)]
            feature["distribution_counts"] = counts_array.tolist()

        else:  # Categorical
            vc = df.lazy().group_by(name).agg(pl.count().alias("counts")).sort("counts", descending=True).collect(engine='gpu')
            values = vc[name].to_list()
            counts = vc["counts"].to_list()
            feature.update({
                "distribution_bins": [str(v) for v in values],
                "distribution_counts": [float(c) for c in counts],
                "mode": str(values[0]) if counts else "",
                "mode_frequency": int(counts[0]) if counts else 0,
                "mode_percent": int(counts[0] / total_rows * 100) if counts else 0,
                "sec_mode": str(values[1]) if len(counts) > 1 else "",
                "sec_mode_frequency": int(counts[1]) if len(counts) > 1 else 0,
                "sec_mode_percent": int(counts[1] / total_rows * 100) if len(counts) > 1 else 0
            })

        features.append(feature)

    return json.dumps(features)





import random
import hashlib
from typing import Any
def category_density_tables(df: pl.DataFrame, category_col: str, target_col: str):
    # contar ocurrencias por (categoria, objetivo)
    agg = (
        df.group_by([category_col, target_col])
          .agg(pl.len().alias("count"))
    )

    # normalizar densidad (0–1) por categoria principal
    agg = agg.with_columns(
        (pl.col("count") / pl.col("count").sum().over(category_col)).alias("density")
    )

    # pivotear para crear matriz densa (categoría vs objetivo)
    pivoted = (
        agg.pivot(values="density", index=category_col, on=target_col, aggregate_function="first")
        .fill_null(0.0)  # por si faltan combinaciones
    )

    # extraer meta (categorías principales)
    meta = pivoted[category_col].to_list()

    # extraer x como lista de listas (solo densidades)
    x = pivoted.drop(category_col).to_numpy().tolist()

    # leyenda (objetivos con colores aleatorios en formato rgb)
    legend = []
    for val in pivoted.drop(category_col).columns:
        legend.append({
            "nombre": val,
            "color": f"{random.randint(0,255)},{random.randint(0,255)},{random.randint(0,255)}"
        })
    now_str = '2025-08-27T15:18:35.883Z'
    print(json.dumps({
            "x": x,
            "meta": meta,
            "leyend": legend,
            "created_at": now_str,
            "updated_at": now_str,
            "status": 0
        }))
    return json.dumps({
        "x": x,
        "meta": meta,
        "leyend": legend,
        "created_at": now_str,
        "updated_at": now_str,
        "status": 0
    })
def _deterministic_rgb(name: str) -> str:
    h = hashlib.md5(name.encode("utf-8")).digest()
    return f"{h[0]},{h[1]},{h[2]}"

def category_boxplot_with_outliers(
    df: pl.DataFrame,
    category_col: str,
    value_col: str
) -> str:
    """
    Devuelve una estructura lista-para-UI con:
      - x: [[whisker_min, q1, median, q3, whisker_max], ...]  (alineado con meta)
      - meta: [categoria1, categoria2, ...]
      - leyend: [{ "nombre": value_col, "color": "rgb(...)" }]
      - outliers: [[v1,v2,...], [...], ...] (lista por categoría, alineada con meta)
    """
    # 1) Validaciones y limpieza básica
    if category_col not in df.columns:
        raise KeyError(f"Columna categórica '{category_col}' no encontrada")
    if value_col not in df.columns:
        raise KeyError(f"Columna continua '{value_col}' no encontrada")

    # Seleccionamos solo las columnas necesarias, casteamos y quitamos nulos
    df_clean = (
        df.lazy().select([category_col, value_col])
          .with_columns(pl.col(value_col).cast(pl.Float64))
          .drop_nulls().collect(engine='gpu')
    )

    if df_clean.height == 0:
        #error return 
        raise ValueError("No hay valores para la columna")

    # 2) Estadísticos por grupo: min, q1, median, q3, max, count
    agg = (
        df_clean.lazy().group_by(category_col)
                .agg([
                    pl.col(value_col).min().alias("min"),
                    pl.col(value_col).quantile(0.25).alias("q1"),
                    pl.col(value_col).median().alias("median"),
                    pl.col(value_col).quantile(0.75).alias("q3"),
                    pl.col(value_col).max().alias("max"),
                    pl.col(value_col).count().alias("count")
                ])
                .sort(category_col).collect(engine='gpu')
    )

    # 3) Calcular IQR y límites de bigotes (lower, upper)
    agg = agg.lazy().with_columns([
        (pl.col("q3") - pl.col("q1")).alias("iqr"),
        (pl.col("q1") - 1.5 * (pl.col("q3") - pl.col("q1"))).alias("lower"),
        (pl.col("q3") + 1.5 * (pl.col("q3") - pl.col("q1"))).alias("upper")
    ]).collect(engine='gpu')

    # 4) Unir límites a las filas para poder filtrar outliers / within
    joined = df_clean.lazy().join(agg.lazy().select([category_col, "lower", "upper"]), on=category_col, how="left").collect(engine='gpu')

    # 5) Whiskers: últimos puntos dentro de [lower, upper] por categoría
    within = (
        joined.filter(
            (pl.col(value_col) >= pl.col("lower")) & (pl.col(value_col) <= pl.col("upper"))
        )
        .group_by(category_col)
        .agg([
            pl.col(value_col).min().alias("whisker_min"),
            pl.col(value_col).max().alias("whisker_max")
        ])
    )

    #
    # 6) Outliers: lista de valores por categoría fuera de los bigotes
    outliers = (
        joined.filter(
            (pl.col(value_col) < pl.col("lower")) | (pl.col(value_col) > pl.col("upper"))
        )
        .group_by(category_col)
        .agg(pl.col(value_col).implode().alias("outliers"))
    )

    # 7) Combinar resultados: agg LEFT JOIN whiskers & outliers
    final = (
        agg.join(within, on=category_col, how="left")
           .join(outliers, on=category_col, how="left")
    )

    # Si whisker_min/whisker_max es null (p.e. no hay puntos dentro del rango), usar min/max del grupo
    final = final.with_columns([
        pl.col("whisker_min").fill_null(pl.col("min")).alias("whisker_min"),
        pl.col("whisker_max").fill_null(pl.col("max")).alias("whisker_max"),
        pl.col("outliers").fill_null(pl.lit([])).list.unique().alias("outliers")
    ])

    # 8) Construir listas alineadas: meta, x y outliers
    meta: list[str] = [str(m) for m in final[category_col].to_list()]
    x_list:list[list[float]] = []
    outliers_list: list[list[float]] = []

    # Asegurar orden consistente (ya está ordenado por category_col)
    for row in final.iter_rows(named=True):
        # Convertir a float nativos de Python
        whisk_min = float(row["whisker_min"])
        q1 = float(row["q1"])
        median = float(row["median"])
        q3 = float(row["q3"])
        whisk_max = float(row["whisker_max"])
        x_list.append([whisk_min, q1, median, q3, whisk_max])

        # outliers viene como lista -> forzar floats
        ol = row["outliers"] or []
        ol_floats = [float(v) for v in ol]
        outliers_list.append(ol_floats)

    leyend = [{"nombre": value_col, "color": _deterministic_rgb(value_col)}]

    return json.dumps({
        "x": x_list,
        "meta": meta,
        "leyend": leyend,
        "outliers": outliers_list
    })
'''{
    # Lista con 5 números por categoría (en el mismo orden que 'meta')
    # Cada sublista es: [whisker_min, q1, median, q3, whisker_max]
    "x": [
        [5.0, 6.5, 7.0, 7.5, 8.0],     # valores resumen para grupo "A"
        [2.0, 3.0, 3.0, 3.0, 3.0],     # valores resumen para grupo "B"
        [10.0, 11.0, 11.5, 12.5, 13.0] # valores resumen para grupo "C"
    ],

    # Nombre de las categorías, alineadas con 'x' y 'outliers'
    "meta": ["A", "B", "C"],

    # Leyenda con información de la variable continua
    # Cada entrada tiene un nombre y un color en formato RGB
    "leyend": [
        {"nombre": "valor", "color": "rgb(220,150,33)"}
    ],

    # Outliers detectados en cada categoría (mismo orden que en 'meta')
    # Si no hay outliers en la categoría, se deja lista vacía
    "outliers": [
        [100.0], # outliers para grupo "A"
        [50.0],  # outliers para grupo "B"
        []       # grupo "C" sin outliers
    ]
}'''
