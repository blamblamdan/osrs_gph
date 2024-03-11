use reqwest::header::HeaderMap;
use std::collections::HashMap;
use toml::Table;

use serde::Deserialize;

#[derive(Debug, Default, Clone)]
pub struct APIHeaders {
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct API<S: AsRef<str>> {
    pub api_url: S, // Url?
    pub headers: APIHeaders,
}

impl<'a, S: AsRef<str> + std::convert::From<&'a str>> Default for API<S> {
    fn default() -> Self {
        API {
            api_url: "127.0.0.1".into(),
            headers: APIHeaders::default(),
        }
    }
}

impl TryFrom<APIHeaders> for HeaderMap {
    type Error = http::Error;
    fn try_from(value: APIHeaders) -> Result<Self, Self::Error> {
        HeaderMap::try_from(&value.headers)
    }
}

// impl<'a, I: Iterator<Item = (&'a String, &'a String)>> FromIterator<I> for APIHeaders{
//     fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self{
//         let h: HashMap<String, String> = HashMap::<String, String>::new();
//         APIHeaders::from(h)
//     }
// }
impl IntoIterator for APIHeaders {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.headers.into_iter()
    }
}

impl Extend<HashMap<String, String>> for APIHeaders {
    fn extend<T: IntoIterator<Item = HashMap<String, String>>>(&mut self, iter: T) {
        for h in iter {
            self.headers.extend(h);
        }
    }
}
impl Extend<(String, String)> for APIHeaders {
    fn extend<T: IntoIterator<Item = (String, String)>>(&mut self, iter: T) {
        self.headers.extend(iter);
    }
}

impl APIHeaders {
    pub fn new<H: Into<HashMap<String, String>>>(headers: H) -> Self {
        APIHeaders {
            headers: headers.into(),
        }
    }
}

impl<H: Into<HashMap<String, String>>> From<H> for APIHeaders {
    // Trait includes Self type when using Into<HashMap<String, String>>
    fn from(headers: H) -> Self {
        APIHeaders {
            headers: headers.into(),
        }
    }
}

pub trait FromTable {
    fn from_table<T: Into<Table>>(t: Table) -> Self;
    fn from_table_ref(t: &Table) -> Self;
}

impl FromTable for APIHeaders {
    fn from_table<T: Into<Table>>(t: Table) -> Self {
        let mut h: HashMap<String, String> = HashMap::new();
        for (k, v) in &t {
            let Ok(value) = String::deserialize(v.clone()) else {
                continue;
            };
            h.insert(k.to_string(), value);
        }
        APIHeaders { headers: h }
    }
    fn from_table_ref(t: &Table) -> Self {
        let mut h: HashMap<String, String> = HashMap::new();
        for (k, v) in t {
            let Ok(value) = String::deserialize(v.clone()) else {
                continue;
            };
            h.insert(k.to_string(), value);
        }
        APIHeaders { headers: h } // h is HashMap<String, String>
    }
}

impl<S: AsRef<str>> API<S> {
    pub fn new<H: Into<APIHeaders>>(api_url: S, headers: H) -> Self {
        API {
            api_url,
            headers: headers.into(),
        }
    }
}
