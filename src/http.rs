use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};

pub async fn one_way<Input: Serialize>(client: &Client, base_url: &str, input: &Input) -> Result<(), crate::Error> {
    let response: String = client.post(base_url)
        .header("Content-Type", "text/xml-SOAP")
        .header("MessageType", "Call")
        .body(to_string(input)?)
        .send()
        .await?.text().await?;

    Ok(())
}

pub async fn request_response<'a, Input: Serialize, Output: Deserialize<'a>, Error: Deserialize<'a>>(client: &Client, base_url: &str, input: &Input)
    -> Result<Result<Output, Error>, crate::Error> {
    let response: String = client.post(base_url)
        .header("Content-Type", "text/xml-SOAP")
        .header("MessageType", "Call")
        .body(to_string(input)?)
        .send()
        .await?.text().await?;

    let res: Result<Output, _> = from_str(&response);

    match res {
        Ok(o) => Ok(Ok(o)),
        Err(e) => match from_str(&response) {
            Err(e) => Err(e.into()),
            Ok(e) => Ok(Err(e)),
        }
    }
}
