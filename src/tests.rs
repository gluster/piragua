use rocket;
use rocket::local::Client;
use rocket::http::{Status, ContentType};

#[test]
fn post_get_put_get() {
    let client = Client::new(rocket()).unwrap();

    // Check that a cluster with ID test doesn't exist.
    let res = client
        .get("/clusters/test")
        .header(ContentType::JSON)
        .dispatch();
    //    assert_eq!(res.status(), Status::NotFound);

    // Create a new cluster
    let res = client
        .post("/clusters")
        .header(ContentType::JSON)
        //.body(r#"{ "contents": "Hello, world!" }"#)
        .dispatch();

    assert_eq!(res.status(), Status::Created);

    /*
    // Check that the message exists with the correct contents.
    let mut res = client
        .get("/message/1")
        .header(ContentType::JSON)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.body().unwrap().into_string().unwrap();
    assert!(body.contains("Hello, world!"));

    // Change the message contents.
    let res = client
        .put("/message/1")
        .header(ContentType::JSON)
        .body(r#"{ "contents": "Bye bye, world!" }"#)
        .dispatch();

    assert_eq!(res.status(), Status::Ok);

    // Check that the message exists with the updated contents.
    let mut res = client
        .get("/message/1")
        .header(ContentType::JSON)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body = res.body().unwrap().into_string().unwrap();
    assert!(!body.contains("Hello, world!"));
    assert!(body.contains("Bye bye, world!"));
    */
}
