
use axum::{
    extract::{Extension, Multipart, Path},
    http::StatusCode,
    response::{IntoResponse, Html, Redirect, Response},
    routing::get,
    Router,
};
use sqlx::{FromRow, sqlite::SqlitePool, types::Uuid};
use std::{
    fs::File,
    sync::Arc,
};

mod db;

#[derive(FromRow)]
struct Post {
    username: String,
    useravatar: Option<Uuid>,
    date: String,             // = YYYY-MM-DDTHH:MM:SSZ
    content: String,
    image: Option<Uuid>,
}

const FRONTPAGE_LOCATION: &str = "/home";
const DATA_LOCATION: &str = "/data";

async fn frontpage(Extension(pool): Extension<Arc<SqlitePool>>) -> Html<String> {
    macro_rules! image {
        ($maybe_uuid:expr) => {
            $maybe_uuid
                .map(|uuid| format!(r#"<img src="{}/{}">"#, DATA_LOCATION, uuid))
                .unwrap_or_else(|| "".to_owned())
        }
    }
    axum::response::Html(format!(
        r###"
        <html>
            <head><title>Blog Posts</title></head>
            <style>
                body {{
                    background-color: #cccccc;
                }}

                .blog-post {{
                    display: flex;
                    width: 100%;
                    border: 1px solid black;
                    margin-top: 5px;
                }}
                
                .user-info {{
                    flex: 0 0 150px;
                    padding: 10px;
                    background-color: #dddddd;
                }}
                .user-info > img {{
                    width: 100%;
                    border: 1px solid black;
                }}
                
                .post-info {{
                    flex: 1;
                    min-height: 200px;
                    padding: 10px;
                    background-color: #eeeeee;
                    box-sizing: border-box;
                }}
                .post-info2 {{
                    display: flex;
                    justify-content: space-between;
                }}
                .post-info > * {{
                    padding: 5px;
                }}
                .post-info > hr {{
                    padding: 0;
                }}
                .post-info > img {{
                    padding: 0px;
                    max-width: 100%;
                }}
            </style>
            <body>
                <h1>Blog Posts</h1>
                <form method="POST" action="#" enctype="multipart/form-data">
                    <input type="text" name="username" placeholder="Username*" required/><br/>
                    <input type="url" name="useravatar" placeholder="User avatar link"/><br/>
                    <textarea name="content" placeholder="Post content*" required></textarea><br/>
                    <input type="file" name="image"/><br/><br/>
                    <button type="submit">Add Post</button>
                </form>
                <div>
                    {}
                </div>
            </body>
            <script>
                window.onload = function() {{
                    for (e of document.getElementsByClassName("post-date")) {{
                        let date = new Date(`${{e.getAttribute("post-date")}}`);
                        e.innerText = `Posted on: ${{date}}`;
                    }}
                }};
            </script>
        </html>
        "###,
        db::fetch_all_posts(pool.as_ref()).await.into_iter().enumerate()
            .map(|(post_order, post)| format!(
            r###"<div class="blog-post" post-order="{}">
                <div class="user-info">{}<span>{}</span></div>
                <div class="post-info"><div class="post-info2"><span class="post-date" post-date="{}">Posted on: ???</span><span>#{}</span></div><hr>
                <p>{}</p>{}</div>
            </div>"###,
            post_order + 1,
            image!(post.useravatar), post.username,
            post.date, post_order + 1, post.content, image!(post.image))).collect::<String>()
    ))
}

async fn add_post(
    Extension(pool): Extension<Arc<SqlitePool>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut username = String::new();
    let mut useravatar_uuid = Option::<Uuid>::None;
    let mut content = String::new();
    let mut image_uuid = Option::<Uuid>::None;
    
    while let Ok(Some(field)) = multipart.next_field().await.map_err(|e| e.to_string()) {
        let field_name = field.name().unwrap_or_default().to_string();

        if field_name == "username" {
            username = match field.text().await.map_err(|e| e.to_string()) {
                Ok(value) => value,
                _ => return (StatusCode::BAD_REQUEST, "bad username".to_owned()).into_response(),
            };
        } else if field_name == "useravatar" {
            let Ok(useravatar_url) = field.text().await.map_err(|e| e.to_string()) else {
                return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response();
            };
            
            if useravatar_url != "" {
                let Ok(useravatar_response) = reqwest::Client::new().get(useravatar_url).send().await else {
                    return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response();
                };
                
                let useravatar_content_type = match useravatar_response.headers().get("Content-Type").map(|e| e.to_str()) {
                    Some(Ok(content_type)) => content_type.to_owned(),
                    _ => return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response(),
                };
                
                let Ok(useravatar_image) = useravatar_response.bytes().await else {
                    return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response();
                };
                
                match db::insert_file(&pool, &useravatar_content_type, useravatar_image.to_vec()).await {
                    Err(_) => return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response(),
                    Ok(uuid) => useravatar_uuid = Some(uuid),
                }
            }
        } else if field_name == "content" {
            content = match field.text().await.map_err(|e| e.to_string()) {
                Ok(value) => value,
                _ => return (StatusCode::BAD_REQUEST, "bad content".to_owned()).into_response(),
            }
        } else if field_name == "image" {
            let image_content_type = field.content_type().unwrap_or("image/png").to_owned();
            
            let image = match field.bytes().await {
                Ok(image) => image.to_vec(),
                Err(e) => {
                    println!("{}", e);
                    return (StatusCode::BAD_REQUEST, "bad image".to_owned()).into_response();
                }
            };
            
            if image.len() != 0 {
                match db::insert_file(&pool, &image_content_type, image).await {
                    Err(_) => return (StatusCode::BAD_REQUEST, "bad image".to_owned()).into_response(),
                    Ok(uuid) => image_uuid = Some(uuid),
                }
            }
        }
    }
    
    if username.is_empty() || content.is_empty() {
        return (StatusCode::BAD_REQUEST, "bad request".to_owned()).into_response();
    }
    
    sqlx::query("INSERT INTO posts (username, useravatar, content, image)
                 VALUES (?, ?, ?, ?)")
        .bind(&username)
        .bind(&useravatar_uuid)
        .bind(&html_escape::encode_text(&content))
        .bind(&image_uuid)
        .execute(pool.as_ref())
        .await
        .unwrap();

    Redirect::to(FRONTPAGE_LOCATION).into_response()
}

async fn serve_data(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    let Ok(file_id) = Uuid::try_parse(&file_id) else {
        return (StatusCode::NOT_FOUND, "invalid id".to_owned()).into_response();
    };

    let Ok((content_type, data)) = db::get_file(&pool, &file_id).await else {
        return (StatusCode::NOT_FOUND, "data not found".to_owned()).into_response();
    };
    
    let mut response = Response::new(data.into());
    response.headers_mut().insert("Content-Type", content_type.parse().unwrap());
    return response;
}

fn print_usage() {
    println!("Usage: {} [OPTIONS]", std::env::args().next().unwrap());
    println!();
    println!("Options:");
    println!("    --help            Print this help message");
    println!("    --db-file <path>  Specify the database file path (optional, runs in memory by default)");
}

// This is strictly speaking not correct, as it will match overlapping arguments
// For use of this task, I thought using clap would be overkill?
fn find_argument<'a>(args: &'a Vec<String>, name: &'a str) -> Option<&'a str> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().map(|x| x.as_str());
        }
    }
    None
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    
    if args.iter().find(|e| *e == "--help").is_some() {
        print_usage();
        return Ok(());
    }
    
    let pool = match find_argument(&args, "--db-file") {
        // Store db to a file
        Some(file_name) => {
            let first_usage = File::open(file_name).is_err();
            
            if first_usage {
                File::create(file_name)?;
            }
            
            let pool = Arc::new(SqlitePool::connect(&format!("sqlite://{}", file_name)).await.unwrap());
            
            if first_usage {
                db::setup_database(&pool).await;
            }
            
            pool
        }
        // Use in-memory db
        None => {
            let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.unwrap());
            db::setup_database(&pool).await;
            pool
        }
    };

    let app = Router::new()
        .route(FRONTPAGE_LOCATION, get(frontpage).post(add_post))
        .route(&format!("{}/:file_id", DATA_LOCATION), get(serve_data))
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    
    Ok(())
}
