use std::fs::File;
use std::io::Read;

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

    // Create a new cluster
    let mut f = File::open("tests/create_volume").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let res = client
        .post("/volumes")
        .header(ContentType::JSON)
        .body(s)
        .dispatch();

    println!("response: {:?}", res);

    assert_eq!(res.status(), Status::Accepted);

    // Check that the message exists with the correct contents.
    let mut res = client
        .get("/volumes/test")
        .header(ContentType::JSON)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    println!(
        "/volumes/test response: {:?}",
        res.body().unwrap().into_string()
    );

    // Change the message contents.
    let mut res = client
        .get("/clusters/cluster-test")
        .header(ContentType::JSON)
        .dispatch();

    assert_eq!(res.status(), Status::Ok);
    println!(
        "/clusters/cluster-test response: {:?}",
        res.body().unwrap().into_string()
    );

    /*
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
