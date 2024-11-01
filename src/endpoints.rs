use axum::{
    extract::{Extension, Multipart, Path},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::{sqlite::SqlitePool, types::Uuid};
use std::{fmt::Write, sync::Arc};

pub async fn frontpage(Extension(pool): Extension<Arc<SqlitePool>>) -> Html<String> {
    macro_rules! image {
        ($maybe_uuid:expr) => {
            $maybe_uuid
                .map(|uuid| format!(r#"<img src="{}/{}">"#, crate::DATA_LOCATION, uuid))
                .unwrap_or_else(|| "".to_owned())
        };
    }

    let html_start = r###"<html>
            <head><title>Blog Posts</title></head>
            <style>
                body {
                    background-color: #cccccc;
                }

                .blog-post {
                    display: flex;
                    width: 100%;
                    border: 1px solid black;
                    margin-top: 5px;
                }
                
                .user-info {
                    flex: 0 0 150px;
                    padding: 10px;
                    background-color: #dddddd;
                }
                .user-info > img {
                    width: 100%;
                    border: 1px solid black;
                }
                
                .post-info {
                    flex: 1;
                    min-height: 200px;
                    padding: 10px;
                    background-color: #eeeeee;
                    box-sizing: border-box;
                }
                .post-info2 {
                    display: flex;
                    justify-content: space-between;
                }
                .post-info > * {
                    padding: 5px;
                }
                .post-info > hr {
                    padding: 0;
                }
                .post-info > img {
                    padding: 0px;
                    max-width: 100%;
                }
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
                <div>"###
        .to_owned();
    let mut html_with_posts = crate::db::fetch_all_posts(pool.as_ref()).await.into_iter().enumerate().fold(html_start, |mut html, (post_order, post)| {
        let _ = write!(html,
            r###"<div class="blog-post" post-order="{}">
                <div class="user-info">{}<span>User: {}</span></div>
                <div class="post-info"><div class="post-info2"><span class="post-date" post-date="{}">Posted on: ???</span><span>#{}</span></div><hr>
                <p>{}</p>{}</div>
            </div>"###, post_order + 1,
            image!(post.useravatar), post.username,
            post.date, post_order + 1, post.content, image!(post.image));
        html
    });
    html_with_posts.push_str(
        r###"
                </div>
            </body>
            <script>
                window.onload = function() {
                    for (e of document.getElementsByClassName("post-date")) {
                        let date = new Date(`${e.getAttribute("post-date")}`);
                        e.innerText = `Posted on: ${date}`;
                    }
                };
            </script>
        </html>
        "###,
    );

    axum::response::Html(html_with_posts)
}

pub async fn add_post(
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

            if !useravatar_url.is_empty() {
                let Ok(useravatar_response) =
                    reqwest::Client::new().get(useravatar_url).send().await
                else {
                    return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response();
                };

                let useravatar_content_type = match useravatar_response
                    .headers()
                    .get("Content-Type")
                    .map(|e| e.to_str())
                {
                    Some(Ok(content_type)) => content_type.to_owned(),
                    _ => {
                        return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned())
                            .into_response()
                    }
                };

                let Ok(useravatar_image) = useravatar_response.bytes().await.map(|e| e.to_vec())
                else {
                    return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned()).into_response();
                };

                if !useravatar_image.is_empty() {
                    match crate::db::insert_file(&pool, &useravatar_content_type, useravatar_image)
                        .await
                    {
                        Err(_) => {
                            return (StatusCode::BAD_REQUEST, "bad user avatar".to_owned())
                                .into_response()
                        }
                        Ok(uuid) => useravatar_uuid = Some(uuid),
                    }
                }
            }
        } else if field_name == "content" {
            content = match field.text().await.map_err(|e| e.to_string()) {
                Ok(value) => value,
                _ => return (StatusCode::BAD_REQUEST, "bad content".to_owned()).into_response(),
            }
        } else if field_name == "image" {
            let image_content_type = field.content_type().unwrap_or("image/png").to_owned();

            let Ok(image) = field.bytes().await.map(|e| e.to_vec()) else {
                return (StatusCode::BAD_REQUEST, "bad image".to_owned()).into_response();
            };

            if !image.is_empty() {
                match crate::db::insert_file(&pool, &image_content_type, image).await {
                    Err(_) => {
                        return (StatusCode::BAD_REQUEST, "bad image".to_owned()).into_response()
                    }
                    Ok(uuid) => image_uuid = Some(uuid),
                }
            }
        }
    }

    if username.is_empty() || content.is_empty() {
        return (StatusCode::BAD_REQUEST, "bad request".to_owned()).into_response();
    }

    match crate::db::insert_post(
        &pool,
        &username,
        &useravatar_uuid,
        &html_escape::encode_text(&content),
        &image_uuid,
    )
    .await
    {
        Err(_) => (StatusCode::BAD_REQUEST, "bad request".to_owned()).into_response(),
        Ok(_) => Redirect::to(crate::FRONTPAGE_LOCATION).into_response(),
    }
}

pub async fn serve_data(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    let Ok(file_id) = Uuid::try_parse(&file_id) else {
        return (StatusCode::NOT_FOUND, "invalid id".to_owned()).into_response();
    };

    let Ok((content_type, data)) = crate::db::get_file(&pool, &file_id).await else {
        return (StatusCode::NOT_FOUND, "data not found".to_owned()).into_response();
    };

    let Ok(content_type) = content_type.parse() else {
        return (StatusCode::NOT_FOUND, "data not found".to_owned()).into_response();
    };

    let mut response = Response::new(data.into());
    response.headers_mut().insert("Content-Type", content_type);
    response
}
