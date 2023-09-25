#[macro_use] extern crate rocket;
use mongodb::results::{InsertOneResult, DeleteResult, UpdateResult};
use mongodb::{Client, options::ClientOptions};
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::serde::{Serialize, Deserialize, json::Json};
use mongodb::bson;
use mongodb::options::FindOneOptions;

// https://www.mongodb.com/developer/languages/rust/serde-improvements/

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Book {
    _id: bson::oid::ObjectId,
    title: String,
    author: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewTaskData {
    name: String,
    frequency: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Task {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct TaskWithLatestEvent {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
    latest_event_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Event {
    _id: bson::oid::ObjectId,
    task: bson::oid::ObjectId,
    date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewEventData {
    task: bson::oid::ObjectId,
    date: String,
}

#[get("/")]
async fn index() -> &'static str {
    return "Hello, world!";
}

async fn read_tasks_list_action() ->Result<Vec<TaskWithLatestEvent>, Error> {
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

    let tasks = db.collection::<Task>("tasks");
    let events = db.collection::<Event>("events");

    let mut cursor = tasks.find(None, None).await?;

    let mut tasks_list = Vec::new();

    while let Some(task) = cursor.try_next().await? {
        println!("task: {:?}", task);
        let filter = bson::doc! {
            "task": task._id,
        };
        let sort_option = bson::doc! {
            "date": -1,
        };
        let options = FindOneOptions::builder().sort(sort_option).build();
        let latest_event = events.find_one(filter, options).await?;
        /* let latest_event_or_null = match latest_event {
            Some(_latest_event) => _latest_event,
            None => continue // I think this prevents the code from progressing beyond this point
        }; */
        // println!("latest_event {:?}", latest_event);

        let task_with_latest_event = match latest_event {
            Some(_latest_event) => TaskWithLatestEvent {
                _id: task._id,
                name: task.name,
                frequency: task.frequency,
                latest_event_date: Some(_latest_event.date),
            },
            None => TaskWithLatestEvent {
                _id: task._id,
                name: task.name,
                frequency: task.frequency,
                latest_event_date: None,
            }
        };
        println!("task_with_latest_event {:?}", task_with_latest_event);
        tasks_list.push(task_with_latest_event);
    }

    println!("tasks_list {:?}", tasks_list);
    Ok(tasks_list)
}

async fn create_task_action(task_data: NewTaskData) -> Result<InsertOneResult, Error> {
    println!("task_data {:?}", task_data);
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

    let tasks = db.collection::<Task>("tasks");

    let new_task = Task {
        _id: bson::oid::ObjectId::new(),
        name: task_data.name,
        frequency: task_data.frequency,
    };
    println!("new_task: {:?}", new_task);

    let task_result = tasks.insert_one(new_task, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn update_task_action(task_data: Task) -> Result<UpdateResult, Error> {
    println!("task_data {:?}", task_data);
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

    let tasks = db.collection::<Task>("tasks");

    let updated_task = bson::doc! {
        "$set": {
            "name": task_data.name,
            "frequency": task_data.frequency,
        }
    };
    println!("new_task: {:?}", updated_task);

    let filter = bson::doc!{"_id": task_data._id };

    let task_result = tasks.update_one(filter, updated_task, None).await;
    println!("task_result {:?}", task_result);

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn delete_tasks_action(tasks_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    println!("task_data {:?}", tasks_data);
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

    let tasks = db.collection::<Task>("tasks");

    let filter = bson::doc!{"_id": { "$in": tasks_data }};

    let tasks_result = tasks.delete_many(filter, None).await;

    match tasks_result {
        Ok(_tasks_result) => Ok(_tasks_result),
        Err(_) => todo!(),
    }
}

async fn create_event_action(event_data: NewEventData) -> Result<InsertOneResult, Error> {
    println!("event_data {:?}", event_data);
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

    let events = db.collection::<Event>("events");

    println!("event_data.date {:?}", event_data.date);
    // let event_date: chrono::DateTime<Utc> = event_data.date.parse().unwrap();

    let new_event = Event {
        _id: bson::oid::ObjectId::new(),
        task: event_data.task,
        date: event_data.date,
    };
    println!("new_event: {:?}", new_event);

    let event_result = events.insert_one(new_event, None).await;

    match event_result {
        Ok(_event_result) => Ok(_event_result),
        Err(_) => todo!(),
    }
}

#[get("/api/tasks", format="json")]
async fn read_tasks_list() -> Result<Json<Vec<TaskWithLatestEvent>>, Status> {
    let tasks = read_tasks_list_action().await;

    match tasks {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tasks", format="json", data="<task>")]
async fn create_task(task: Json<NewTaskData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_task = task.into_inner();
    println!("deserialized_task {:?}", deserialized_task);
    let task = create_task_action(deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/tasks", format="json", data="<task>")]
async fn update_task(task: Json<Task>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_task = task.into_inner();
    println!("deserialized_task {:?}", deserialized_task);
    let task = update_task_action(deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tasks", format="json", data="<tasks>")]
async fn delete_tasks(tasks: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tasks_list = tasks.into_inner();
    println!("deserialized_tasks {:?}", deserialized_tasks_list);
    let tasks = delete_tasks_action(deserialized_tasks_list).await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/events", format="json", data="<event>")]
async fn create_event(event: Json<NewEventData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_event = event.into_inner();
    println!("deserialized_event {:?}", deserialized_event);
    let event = create_event_action(deserialized_event).await;

    match event {
        Ok(event_result) => Ok(Json(event_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![read_tasks_list])
        .mount("/", routes![create_task])
        .mount("/", routes![update_task])
        .mount("/", routes![delete_tasks])
        .mount("/", routes![create_event])
}