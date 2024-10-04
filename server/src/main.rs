use axum::extract::Path;
use axum::extract::Query;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::response::Response;
use axum::routing::get;
use clap::Parser;
use md5::Digest as _;
use md5::Md5;
use sigil_rs::Sigil;
use sigil_rs::Theme;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

const MAX_WIDTH: u32 = 600;

fn default_width() -> u32 {
    120
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ImageQuery {
    #[serde(rename = "w", default = "default_width")]
    width: u32,
    #[serde(default)]
    inverted: bool,
}

#[axum::debug_handler]
async fn handler(
    headers: HeaderMap,
    Query(query): Query<ImageQuery>,
    path: Option<Path<String>>,
) -> Response {
    let theme = Theme::default();

    let path = path.map_or(String::new(), |path| path.0);

    if query.width > MAX_WIDTH {
        return (
            StatusCode::BAD_REQUEST,
            format!("Invalid w parameter, must be less than {MAX_WIDTH}"),
        )
            .into_response();
    }
    let div = u32::from(theme.rows + 1) * 2;
    if query.width % div != 0 {
        return (
            StatusCode::BAD_REQUEST,
            format!("Invalid w parameter, must be evenly divisible by {div}"),
        )
            .into_response();
    }

    let hash = if path.len() == 32 && path.bytes().all(|b| b.is_ascii_hexdigit()) {
        std::array::from_fn(|index| {
            let s = &path[index * 2..index * 2 + 2];
            u8::from_str_radix(s, 16).unwrap_or_default()
        })
        .into()
    } else {
        let mut hash = Md5::new();
        hash.update(&path);
        hash.finalize()
    };
    let etag = format!("{hash:x}");
    if let Some(if_none_match) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    {
        if if_none_match.contains(&etag) {
            return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag.as_str())]).into_response();
        }
    }

    let sigil = Sigil::from_hash(&theme, hash.into());
    let sigil = if query.inverted {
        sigil.invert()
    } else {
        sigil
    };

    let image = sigil.to_image(query.width);
    let mut encoded = std::io::Cursor::new(vec![]);
    if let Err(err) = image.write_to(&mut encoded, image::ImageFormat::Png) {
        return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
    };
    let encoded = encoded.into_inner();

    let headers = [
        (header::ETAG, etag.as_str()),
        (header::CACHE_CONTROL, "max-age=315360000"),
        (header::CONTENT_TYPE, "image/png"),
    ];

    (headers, encoded).into_response()
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long, short, env = "HOST", default_value = "127.0.0.1")]
    host: String,
    #[arg(long, short, env = "PORT", default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_default())
        .with(tracing_subscriber::fmt::layer().event_format(tracing_subscriber::fmt::format()))
        .init();

    let args = Args::parse();

    let router = axum::Router::new()
        .route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
        .route("/:path", get(handler))
        .route("/", get(handler))
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind((args.host, args.port)).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, router).await?;

    Ok(())
}
