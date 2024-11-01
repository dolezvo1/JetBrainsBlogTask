
use sqlx::{sqlite::SqlitePool, types::Uuid, FromRow};

#[derive(FromRow)]
pub struct Post {
    pub username: String,
    pub useravatar: Option<Uuid>,
    pub date: String, // = YYYY-MM-DDTHH:MM:SSZ
    pub content: String,
    pub image: Option<Uuid>,
}

pub async fn setup_database(pool: &SqlitePool) {
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            useravatar TEXT,
            date TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc')),
            content TEXT NOT NULL,
            image TEXT
        );
        CREATE TABLE files (
            id BLOB PRIMARY KEY,
            content BLOB NOT NULL,
            content_type TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => panic!("Database initialization failed: {}", e),
    }
}

pub async fn insert_post(
    pool: &SqlitePool,
    username: &str,
    useravatar_uuid: &Option<Uuid>,
    content: &str,
    image_uuid: &Option<Uuid>,
) -> Result<(), ()> {
    sqlx::query(
        "INSERT INTO posts (username, useravatar, content, image)
                 VALUES (?, ?, ?, ?)",
    )
    .bind(username)
    .bind(useravatar_uuid)
    .bind(content)
    .bind(image_uuid)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|_| ())
}

pub async fn fetch_all_posts(pool: &SqlitePool) -> Vec<Post> {
    match sqlx::query_as::<_, Post>("SELECT * FROM posts")
        .fetch_all(pool)
        .await
    {
        Ok(a) => a,
        Err(_) => vec![],
    }
}

pub async fn insert_file(
    pool: &SqlitePool,
    content_type: &str,
    content: Vec<u8>,
) -> Result<Uuid, ()> {
    let uuid = Uuid::now_v7();

    match sqlx::query(
        "INSERT INTO files (id, content_type, content)
                 VALUES (?, ?, ?)",
    )
    .bind(uuid)
    .bind(content_type)
    .bind(content)
    .execute(pool)
    .await
    {
        Ok(_) => Ok(uuid),
        Err(_) => Err(()),
    }
}

pub async fn get_file(pool: &SqlitePool, uuid: &Uuid) -> Result<(String, Vec<u8>), ()> {
    match sqlx::query_as::<_, (String, Vec<u8>)>(
        "SELECT content_type, content FROM files WHERE id = ?",
    )
    .bind(uuid)
    .fetch_one(pool)
    .await
    {
        Ok(a) => Ok(a),
        Err(_) => Err(()),
    }
}
