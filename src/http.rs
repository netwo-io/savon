use crate::gen::{FromElement, ToElements};
use crate::rpser::{Method, Response};
use reqwest::Client;
use std::fmt::Debug;

pub async fn one_way<Input: ToElements>(
    client: &Client,
    base_url: &str,
    ns: &str,
    method: &str,
    input: &Input,
) -> Result<(), crate::Error> {
    let mut v = input.to_elements();
    let mut m = Method::new(method);

    for el in v.drain(..) {
        m = m.with(el);
    }
    let s = m.as_xml(ns);
    trace!("sending: {}", s);

    let response: String = client
        .post(base_url)
        .header("Content-Type", "text/xml")
        .header("MessageType", "Call")
        .body(s)
        .send()
        .await?
        .text()
        .await?;

    println!("received: {}", response);
    Ok(())
}

pub async fn request_response<Input: ToElements, Output: Debug + FromElement, Error>(
    client: &Client,
    base_url: &str,
    ns: &str,
    method: &str,
    input: &Input,
) -> Result<Result<Output, Error>, crate::Error> {
    let mut v = input.to_elements();
    let mut m = Method::new(method);

    for el in v.drain(..) {
        m = m.with(el);
    }
    let s = m.as_xml(ns);
    trace!("sending: {}", s);

    let response: String = client
        .post(base_url)
        .header("Content-Type", "text/xml")
        .header("MessageType", "Call")
        .body(s)
        .send()
        .await?
        .text()
        .await?;

    trace!("received: {}", response);
    let r = Response::from_xml(&response).unwrap();
    trace!("parsed: {:#?}", r);
    let o = Output::from_element(&r.body);
    trace!("output: {:#?}", o);

    o.map(Ok)
}
