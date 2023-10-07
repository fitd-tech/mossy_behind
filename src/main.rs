#[macro_use] extern crate rocket;
use mongodb::results::{InsertOneResult, DeleteResult, UpdateResult};
use mongodb::{Client, options::ClientOptions};
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::serde::{Serialize, Deserialize, json::Json};
use mongodb::bson;
use mongodb::options::{FindOptions, FindOneOptions};

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
struct Tag {
    _id: bson::oid::ObjectId,
    name: String,
    description: Option<String>,
    parent_tag: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewTagData {
    name: String,
    description: Option<String>,
    parent_tag: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewTaskData {
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Task {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct TaskWithLatestEvent {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>,
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
struct EventWithStringValues {
    _id: bson::oid::ObjectId,
    task: Option<String>,
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

async fn read_tasks_action() ->Result<Vec<TaskWithLatestEvent>, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tasks = db.collection::<Task>("tasks");
    let events = db.collection::<Event>("events");

    let mut cursor = tasks.find(None, None).await?;

    let mut tasks_list = Vec::new();

    while let Some(task) = cursor.try_next().await? {
        let filter = bson::doc! {
            "task": task._id,
        };
        let sort_option = bson::doc! {
            "date": -1,
        };
        let options = FindOneOptions::builder().sort(sort_option).build();
        let latest_event = events.find_one(filter, options).await?;

        let task_with_latest_event = match latest_event {
            Some(_latest_event) => TaskWithLatestEvent {
                _id: task._id,
                name: task.name,
                frequency: task.frequency,
                tags: task.tags,
                latest_event_date: Some(_latest_event.date),
            },
            None => TaskWithLatestEvent {
                _id: task._id,
                name: task.name,
                frequency: task.frequency,
                tags: task.tags,
                latest_event_date: None,
            }
        };
        tasks_list.push(task_with_latest_event);
    }

    Ok(tasks_list)
}

async fn read_events_action() ->Result<Vec<Event>, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let events = db.collection::<Event>("events");

    let sort_option = bson::doc! {
        "date": -1,
    };
    let options = FindOptions::builder().sort(sort_option).build();
    let mut cursor = events.find(None, options).await?;

    let mut events_list = Vec::new();

    while let Some(event) = cursor.try_next().await? {
        events_list.push(event);
    }

    Ok(events_list)
}

async fn read_events_string_action() ->Result<Vec<EventWithStringValues>, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let events = db.collection::<Event>("events");
    let tasks = db.collection::<Task>("tasks");

    let sort_option = bson::doc! {
        "date": -1,
    };
    let options = FindOptions::builder().sort(sort_option).build();
    let mut cursor = events.find(None, options).await?;

    let mut events_list = Vec::new();

    while let Some(event) = cursor.try_next().await? {
        let filter = bson::doc! {
            "_id": event.task,
        };
        let task = tasks.find_one(filter, None).await?;

        let event_with_string_values = match task {
            Some(_task) => EventWithStringValues {
                _id: event._id,
                task: Some(_task.name),
                date: event.date,
            },
            None => EventWithStringValues {
                _id: event._id,
                task: None,
                date: event.date,
            }
        };
        events_list.push(event_with_string_values);
    }

    Ok(events_list)
}

async fn read_tags_action() ->Result<Vec<Tag>, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tags = db.collection::<Tag>("tags");

    let sort_option = bson::doc! {
        "name": 1,
    };
    let options = FindOptions::builder().sort(sort_option).build();
    let mut cursor = tags.find(None, options).await?;

    let mut tags_list = Vec::new();

    while let Some(tag) = cursor.try_next().await? {
        tags_list.push(tag);
    }

    Ok(tags_list)
}

async fn create_task_action(task_data: NewTaskData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tasks = db.collection::<Task>("tasks");

    let new_task = Task {
        _id: bson::oid::ObjectId::new(),
        name: task_data.name,
        frequency: task_data.frequency,
        tags: task_data.tags,
    };

    let task_result = tasks.insert_one(new_task, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn create_event_action(event_data: NewEventData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let events = db.collection::<Event>("events");

    let new_event = Event {
        _id: bson::oid::ObjectId::new(),
        task: event_data.task,
        date: event_data.date,
    };

    let event_result = events.insert_one(new_event, None).await;

    match event_result {
        Ok(_event_result) => Ok(_event_result),
        Err(_) => todo!(),
    }
}

async fn create_tag_action(tag_data: NewTagData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tags = db.collection::<Tag>("tags");

    let new_tag = Tag {
        _id: bson::oid::ObjectId::new(),
        name: tag_data.name,
        description: tag_data.description,
        parent_tag: tag_data.parent_tag,
    };

    let tag_result = tags.insert_one(new_tag, None).await;

    match tag_result {
        Ok(_tag_result) => Ok(_tag_result),
        Err(_) => todo!(),
    }
}

async fn update_task_action(task_data: Task) -> Result<UpdateResult, Error> {
    println!("task_data {:?}", task_data);
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tasks = db.collection::<Task>("tasks");

    let updated_task = bson::doc! {
        "$set": {
            "name": task_data.name,
            "frequency": task_data.frequency,
            "tags": task_data.tags,
        }
    };

    let filter = bson::doc!{"_id": task_data._id };

    let task_result = tasks.update_one(filter, updated_task, None).await;
    println!("task_result {:?}", task_result);

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn update_event_action(event_data: EventWithStringValues) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let events = db.collection::<Event>("events");

    let updated_event = bson::doc! {
        "$set": {
            "date": event_data.date,
        }
    };

    let filter = bson::doc!{"_id": event_data._id };

    let event_result = events.update_one(filter, updated_event, None).await;

    match event_result {
        Ok(_event_result) => Ok(_event_result),
        Err(_) => todo!(),
    }
}

async fn update_tag_action(tag_data: Tag) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tags = db.collection::<Tag>("tags");

    let updated_tag = bson::doc! {
        "$set": {
            "name": tag_data.name,
            "description": tag_data.description,
            "parent_tag": tag_data.parent_tag,
        }
    };

    let filter = bson::doc!{"_id": tag_data._id };

    let tag_result = tags.update_one(filter, updated_tag, None).await;

    match tag_result {
        Ok(_tag_result) => Ok(_tag_result),
        Err(_) => todo!(),
    }
}

async fn delete_tasks_action(tasks_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tasks = db.collection::<Task>("tasks");

    let filter = bson::doc!{"_id": { "$in": tasks_data }};

    let tasks_result = tasks.delete_many(filter, None).await;

    match tasks_result {
        Ok(_tasks_result) => Ok(_tasks_result),
        Err(_) => todo!(),
    }
}

async fn delete_events_action(events_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let events = db.collection::<Event>("events");

    let filter = bson::doc!{"_id": { "$in": events_data }};

    let events_result = events.delete_many(filter, None).await;

    match events_result {
        Ok(_events_result) => Ok(_events_result),
        Err(_) => todo!(),
    }
}

async fn delete_tags_action(tags_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let tags = db.collection::<Tag>("tags");

    let filter = bson::doc!{"_id": { "$in": tags_data }};

    let tags_result = tags.delete_many(filter, None).await;

    match tags_result {
        Ok(_tags_result) => Ok(_tags_result),
        Err(_) => todo!(),
    }
}

#[get("/api/tasks", format="json")]
async fn read_tasks() -> Result<Json<Vec<TaskWithLatestEvent>>, Status> {
    let tasks = read_tasks_action().await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tasks", format="json", data="<task>")]
async fn create_task(task: Json<NewTaskData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_task = task.into_inner();
    let task = create_task_action(deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/tasks", format="json", data="<task>")]
async fn update_task(task: Json<Task>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_task = task.into_inner();
    let task = update_task_action(deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tasks", format="json", data="<tasks>")]
async fn delete_tasks(tasks: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tasks_list = tasks.into_inner();
    let tasks = delete_tasks_action(deserialized_tasks_list).await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/events", format="json")]
async fn read_events() -> Result<Json<Vec<Event>>, Status> {
    let events = read_events_action().await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/events-string", format="json")]
async fn read_events_string() -> Result<Json<Vec<EventWithStringValues>>, Status> {
    let events = read_events_string_action().await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/events", format="json", data="<event>")]
async fn create_event(event: Json<NewEventData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_event = event.into_inner();
    let event = create_event_action(deserialized_event).await;

    match event {
        Ok(event_result) => Ok(Json(event_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/events", format="json", data="<event>")]
async fn update_event(event: Json<EventWithStringValues>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_event = event.into_inner();
    let event = update_event_action(deserialized_event).await;

    match event {
        Ok(event_result) => Ok(Json(event_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/events", format="json", data="<events>")]
async fn delete_events(events: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_events_list = events.into_inner();
    let events = delete_events_action(deserialized_events_list).await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/tags", format="json")]
async fn read_tags() -> Result<Json<Vec<Tag>>, Status> {
    let tags = read_tags_action().await;

    match tags {
        Ok(tags_result) => Ok(Json(tags_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tags", format="json", data="<tag>")]
async fn create_tag(tag: Json<NewTagData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_tag = tag.into_inner();
    let tag = create_tag_action(deserialized_tag).await;

    match tag {
        Ok(tag_result) => Ok(Json(tag_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/tags", format="json", data="<tag>")]
async fn update_tag(tag: Json<Tag>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_tag = tag.into_inner();
    let tag = update_tag_action(deserialized_tag).await;

    match tag {
        Ok(tag_result) => Ok(Json(tag_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tags", format="json", data="<tags>")]
async fn delete_tags(tags: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tags_list = tags.into_inner();
    let tags = delete_tags_action(deserialized_tags_list).await;

    match tags {
        Ok(tags_result) => Ok(Json(tags_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![read_tasks])
        .mount("/", routes![create_task])
        .mount("/", routes![update_task])
        .mount("/", routes![delete_tasks])
        .mount("/", routes![read_events])
        .mount("/", routes![read_events_string])
        .mount("/", routes![create_event])
        .mount("/", routes![update_event])
        .mount("/", routes![delete_events])
        .mount("/", routes![read_tags])
        .mount("/", routes![create_tag])
        .mount("/", routes![update_tag])
        .mount("/", routes![delete_tags])
}