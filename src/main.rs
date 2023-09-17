#[macro_use] extern crate rocket;
use mongodb::{Client, options::ClientOptions};
// use mongodb::bson::extjson::de::Error;
// use mongodb::bson::{doc, Document};
use mongodb::Collection;
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::serde::{Serialize, Deserialize, json::Json};

// https://www.mongodb.com/developer/languages/rust/serde-improvements/

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Book {
    title: String,
    author: String,
}

// #[tokio::main]
async fn mongo() -> Result<Option<Book>, Error> {
    println!("Hello, world!");

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;

    client_options.app_name = Some("mossy".to_string());

    let client = Client::with_options(client_options)?;

    for db_name in client.list_database_names(None, None).await? {
        println!("db_name: {}", db_name);
    }

    let db = client.database("mossy");

    for collection_name in db.list_collection_names(None).await? {
        println!("collection_name: {}", collection_name);
    }

    let books = db.collection::<Book>("books");

    let docs = vec![    
        Book { title: "1984".to_string(), author: "George Orwell".to_string() },
        Book { title: "Animal Farm".to_string(), author: "George Orwell".to_string() },
        Book { title: "The Great Gatsby".to_string(), author: "F. Scott Fitzgerald".to_string() },
    ];

    books.insert_many(docs, None).await?;

    let mut cursor = books.find(None, None).await?;

    while let Some(book) = cursor.try_next().await? {
        println!("book: {:?}", book);
    }

    let book = books.find_one(None, None).await;

    match book {
        Ok(book_result) => Ok(book_result),
        Err(_) => todo!(),
    }
}

#[get("/")]
async fn index() -> &'static str {
    return "Hello, world!";
}

#[get("/api/books", format="json")]
async fn get_books_list() -> Result<Json<Option<Book>>, Status> {
    let book = mongo().await;
    println!("Found book:");
    println!("book {:?}", book);

    /* let book: Result<Option<Book>, Error> = books?.find_one(None, None).await;

    match books {
        Ok(book_result) => Ok(Json(book_result)),
        Err(_) => Err(Status::InternalServerError),
    } */

    match book {
        Ok(book_result) => Ok(Json(book_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![get_books_list])
}