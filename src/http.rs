use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};
use crate::gen::ToElement;

pub async fn one_way<Input: Serialize>(client: &Client, base_url: &str, method: &str, input: &Input) -> Result<(), crate::Error> {
    let s = to_string(input)?;
    println!("sending: {:?}", s);

    let response: String = client.post(base_url)
        .header("Content-Type", "text/xml")
        .header("MessageType", "Call")
        .body(s)
        .send()
        .await?.text().await?;

    println!("received: {}", response);
    Ok(())
}

pub async fn request_response<'a, Input: ToElement+Serialize, Output: Deserialize<'a>, Error: Deserialize<'a>>(client: &Client, base_url: &str, method: &str, input: &Input)
    -> Result<Result<Output, Error>, crate::Error> {
    let s = input.to_element();
    let s = crate::rpser::Method::new(method).with(s).as_xml("http://hello.com");
    println!("sending: {}", s);

    let response: String = client.post(base_url)
        .header("Content-Type", "text/xml")
        .header("MessageType", "Call")
        .body(s)
        .send()
        .await?.text().await?;

    println!("received: {}", response);
    let res: Result<Output, _> = from_str(&response);

    match res {
        Ok(o) => Ok(Ok(o)),
        Err(e) => match from_str(&response) {
            Err(e) => Err(e.into()),
            Ok(e) => Ok(Err(e)),
        }
    }
}
