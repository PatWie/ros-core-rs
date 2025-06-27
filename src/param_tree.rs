use std::{collections::HashMap, mem};

use dxr::{TryFromValue, TryToValue, Value};

#[derive(Debug, PartialEq)]
pub enum ParamValue {
    HashMap(HashMap<String, ParamValue>),
    Array(Vec<ParamValue>),
    Value(Value),
}

impl From<&Value> for ParamValue {
    fn from(value: &Value) -> Self {
        if let Ok(hm) = HashMap::<String, Value>::try_from_value(value) {
            let mut rv = HashMap::with_capacity(hm.len());
            for (k, v) in hm.into_iter() {
                rv.insert(k, ParamValue::from(&v));
            }
            return Self::HashMap(rv);
        }
        if let Ok(vec) = Vec::<Value>::try_from_value(value) {
            let mut rv = Vec::with_capacity(vec.len());
            for e in vec.into_iter() {
                rv.push(ParamValue::from(&e))
            }
            return Self::Array(rv);
        }
        Self::Value(value.clone())
    }
}

impl TryToValue for ParamValue {
    fn try_to_value(&self) -> Result<Value, dxr::DxrError> {
        match self {
            ParamValue::Value(v) => Ok(v.clone()),
            ParamValue::Array(arr) => arr.try_to_value(),
            ParamValue::HashMap(hm) => hm.try_to_value(),
        }
    }
}

impl ParamValue {
    pub(crate) fn get_keys(&self) -> Vec<String> {
        match self {
            ParamValue::HashMap(hm) => {
                let mut keys = Vec::new();
                for (k, v) in hm.iter() {
                    keys.push(format!("/{k}"));
                    for suffix in v.get_keys() {
                        keys.push(format!("/{k}{suffix}"));
                    }
                }
                keys
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn contains(&self, key: String) -> bool {
        let key = key.split('/');
        self.get(key).is_some()
    }

    pub(crate) fn get<I, T>(&self, key: I) -> Option<Value>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut hm = self;
        for e in key.into_iter() {
            let e = e.as_ref();
            if e == "" {
                continue;
            }
            match hm {
                ParamValue::HashMap(inner) => {
                    if let Some(inner_value) = inner.get(e) {
                        hm = inner_value;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(hm.try_to_value().unwrap())
    }

    pub(crate) fn remove<I, T>(&mut self, key: I)
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut peekable = key.into_iter().peekable();
        match self {
            ParamValue::HashMap(inner) => {
                let mut hm = inner;
                loop {
                    let current_key = peekable.next();
                    let next_key = peekable.peek();
                    match (current_key, next_key) {
                        (Some(current_key), None) => {
                            hm.remove(current_key.as_ref());
                            return;
                        }
                        (None, None) => {
                            let _ = mem::replace(self, ParamValue::HashMap(hashmap! {}));
                            return;
                        }
                        (None, Some(_)) => unreachable!(),
                        (Some(current_key), Some(_)) => match hm.get_mut(current_key.as_ref()) {
                            Some(ParamValue::HashMap(new_hm)) => hm = new_hm,
                            _ => return,
                        },
                    }
                }
            }
            _ => (),
        }
    }

    pub(crate) fn update_inner<I, T>(&mut self, mut key: I, value: Value)
    where
        I: Iterator<Item = T>,
        T: AsRef<str>,
    {
        match key.next() {
            None => {
                let _ = mem::replace(self, ParamValue::from(&value));
            }
            Some(next_key) => match self {
                ParamValue::HashMap(hm) => match hm.get_mut(next_key.as_ref()) {
                    Some(inner) => inner.update_inner(key, value),
                    None => {
                        hm.insert(next_key.as_ref().to_string(), {
                            let mut inner = ParamValue::HashMap(HashMap::new());
                            inner.update_inner(key, value);
                            inner
                        });
                    }
                },
                _ => {
                    let mut inner = ParamValue::HashMap(hashmap! {});
                    inner.update_inner(key, value);
                    let outer = ParamValue::HashMap(hashmap! {
                        next_key.as_ref().to_string() => inner
                    });
                    let _ = mem::replace(self, outer);
                }
            },
        }
    }
}

use maplit::hashmap;

#[test]
fn test_param_tree() {
    let mut tree = ParamValue::HashMap(hashmap! {
        "run_id".to_owned() => ParamValue::Value(Value::string("asdf-jkl0".to_owned())),
        "robot_id".to_owned() => ParamValue::Value(Value::i4(42)),
        "robot_configs".to_owned() => ParamValue::Array(vec![
            ParamValue::HashMap(hashmap! {
                "robot_speed".to_owned() => ParamValue::Value(Value::double(3.0)),
                "robot_id".to_owned() => ParamValue::Value(Value::i4(24))
            })
        ]),
        "arms".to_owned() => ParamValue::HashMap(hashmap! {
            "arm_left".to_owned() => ParamValue::HashMap(hashmap! {
                "length".to_owned() => ParamValue::Value(Value::double(-0.45))
            })
        })
    });

    tree.update_inner(["robot_configs"].iter(), Value::i4(23));
    let res = tree.get(["robot_configs"]).unwrap();
    assert_eq!(res, Value::i4(23));

    assert!(tree.contains("/".to_owned()));
    assert!(tree.contains("/arms".to_owned()));
    assert!(tree.contains("/arms/arm_left".to_owned()));
}
