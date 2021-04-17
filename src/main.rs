extern crate base64;
extern crate reqwest;
extern crate serde;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{thread, time};
//use serde_json::Result;

use std::fs::OpenOptions;
use std::io::prelude::*;

use error_chain::error_chain;
use std::fs;
use std::fs::File;
use std::io::copy;
use tempfile::Builder;

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}

#[derive(Serialize, Deserialize, Debug)]
struct Versions {
  canary: String,
  dev: String,
  beta: String,
  stable: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hashes {
  Sha1: String,
  Sha256: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Releases {
  FileId: String,
  Url: String,
  SizeInBytes: u32,
  Hashes: Hashes,
}

#[tokio::main]
async fn main() -> Result<()> {
  let client = Client::builder()
    .user_agent("Microsoft Edge Update/1.3.139.59;winhttp")
    .danger_accept_invalid_certs(true)
    .http1_title_case_headers()
    .build()?;

  let versions = client
    .get("https://www.microsoftedgeinsider.com/api/versions")
    .send()
    .await?
    .json::<Versions>()
    .await?;

  // println!("{:?}", versions);

  let mut releases = HashMap::new();
  releases.insert(String::from("stable"), versions.stable);
  releases.insert(String::from("dev"), versions.dev);
  releases.insert(String::from("canary"), versions.canary);
  releases.insert(String::from("beta"), versions.beta);
  let edge_url="https://msedge.api.cdp.microsoft.com/api/v1.1/internal/contents/Browser/namespaces/Default/names/msedge-";

  for (ring, ringversion) in &releases {
    let combined = [
      edge_url,
      ring,
      &String::from("-win-x64/versions/"),
      ringversion,
      &String::from("/files?action=GenerateDownloadInfo&foregroundPriority=true"),
    ]
    .concat();

    // println!("{}", combined);

    let m: HashMap<&str, &str> = HashMap::new();

    let x = client
      .post(combined)
      .json(&m)
      .send()
      .await?
      //.json::<Releases>()
      .text()
      .await?;

    // println!("{:?}", x);

    let array: Vec<Releases> = serde_json::from_str(&x).unwrap();

    for elem in array.iter() {
      let tofind = ["MicrosoftEdge_X64_", ringversion, ".exe"].concat();
      let fname = ["MicrosoftEdge_X64_", ring, "_", ringversion, ".exe"].concat();
      if elem.FileId == tofind {
        // println!("{} {}", elem.Hashes.Sha256, fname);

        let b64bytes = base64::decode(&elem.Hashes.Sha256).unwrap();
        let hexsum = hex::encode(b64bytes);

        let output = [&hexsum, "\t", &fname, "\n"].concat();

        let mut file = OpenOptions::new()
          .create(true)
          .append(true)
          .open("sha256sum.txt")?;
        file.write_all(output.as_bytes())?;
        file.flush()?;

        let mut ringurl = OpenOptions::new()
          .create(true)
          .write(true)
          .open([ring, ".txt"].concat())?;

        ringurl.write_all(elem.Url.as_bytes())?;
        ringurl.flush()?;
        //download(&elem.Url, &fname).await;
      }
    }

    thread::sleep(time::Duration::from_secs(30));
    println!("next file");
  }
  Ok(())
}

async fn download(target: &str, name: &str) -> Result<()> {
  let response = reqwest::get(target).await?;
  let mut dest = File::create(name)?;
  let content = response.text().await?;

  copy(&mut content.as_bytes(), &mut dest)?;
  Ok(())
}
