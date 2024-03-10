use serde::{Serialize, Deserialize, Deserializer};
use std::collections::HashMap;
use serde::de::Visitor;
use std::fmt;
use serde::ser::SerializeStruct;

#[derive(Debug,Clone, Default, Copy)]
pub struct PriceDatum {
    pub high: Option<i32>,
    pub high_time: Option<i32>, // Unix Timestamp
    pub low: Option<i32>,
    pub low_time: Option<i32>
}

#[derive(Debug,Clone, Default)]
pub struct PriceDataType { 
    pub data: HashMap<String, PriceDatum>
}

impl <'de> Deserialize<'de> for PriceDatum {
    // #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PriceDatumVisitor;

        impl <'de> Visitor<'de> for PriceDatumVisitor {
            type Value = PriceDatum;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("four fields containting values/nones: i32,u32,i32,u32.")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>, {
                // While there are entries remaining in the input, add them
                // into our map.
                let mut datum = HashMap::<String, Option<i32>>::with_capacity(4);
                while let Some((key, value)) = map.next_entry::<String, Option<i32>>()? {
                    datum.insert(key, value);
                };
                
                
        
                Ok(PriceDatum{
                    high: datum["high"],
                    high_time: datum["highTime"],
                    low: datum["low"],
                    low_time: datum["lowTime"]
                })
            }
            
        }
        deserializer.deserialize_map(PriceDatumVisitor)
    }
    
}

impl<'de> Deserialize<'de> for PriceDataType {
    // #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PriceDataTypeVisitor;

        impl<'de> Visitor<'de> for PriceDataTypeVisitor {
            type Value = PriceDataType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("field `data` containing <String, PriceDatum> key-value pairs")
            }

            // #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>, {

                // While there are entries remaining in the input, add them
                // into our map.
                let mut api_data = HashMap::<String, HashMap<String, PriceDatum>>::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry()? {
                    api_data.insert(key, value); // Should just insert "data" key
                }

                let mut a_data = api_data["data"].clone().into_iter();
                let mut data = HashMap::<String, PriceDatum>::with_capacity((&api_data["data"]).capacity());
                while let Some((key, value)) = a_data.next() {
                    data.insert(key, value); // Inserting each "id" => {...}
                }
        
                Ok(PriceDataType{data})
            }
        }
        deserializer.deserialize_map(PriceDataTypeVisitor)
    }
}


// This might be slower than the default implementation
impl Serialize for PriceDatum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("PriceDatum", 4)?;
        // TODO: This will error when switching to volume
        state.serialize_field::<Option<i32>>("high",  &self.high)?;
        state.serialize_field::<Option<i32>>("highTime", &self.high_time)?;
        state.serialize_field::<Option<i32>>("low", &self.low)?;
        state.serialize_field::<Option<i32>>("lowTime", &self.low_time)?;
        state.end()
    }
}

impl Serialize for PriceDataType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("PriceDataType", 1)?;
        state.serialize_field::<HashMap<String, PriceDatum>>("data", &self.data)?;
        state.end()
    }
}

impl PriceDatum {
    pub fn invalid_data(&self) -> bool{ // Not valid if an item's field is None
        self.high.is_none() || self.high_time.is_none() || self.low.is_none() || self.low_time.is_none()
    }
}