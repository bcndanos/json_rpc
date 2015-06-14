extern crate asynchronous;
extern crate rustc_serialize;

pub use rustc_serialize as serialize;

use asynchronous::Deferred;
use std::collections::BTreeMap;
use std::sync::Arc;
pub use serialize::json::Json;

pub struct Error {
    code : i64,
    message : String,
    data : Option<Json>,
}

impl Error {
    pub fn custom(code:i64, message: &str, data: Option<Json>) -> Error {
        if code >= -32768 && code <= -32000 {
            panic!("You cannot assign a pre-defined error.");
        }
        Error {
            code: code, message: message.to_string(), data: data
        }
    }

    pub fn predefined(code:i64, data: Option<Json>) -> Error {
        Error {
            code: code, 
            message:  match code {
                -32700 => "Parse error".to_string(),
                -32600 => "Invalid Request".to_string(),
                -32601 => "Method not found".to_string(),
                -32602 => "Invalid params".to_string(),
                -32603 => "Internal error".to_string(),
                -32099 ... -32000 => "Server error".to_string(),
                _ => panic!("Predefined error code incorrect.")
            }, 
            data: data
        }
    }

    fn as_object(&self) -> Json {
        let mut error_object = BTreeMap::new();
        error_object.insert("code".to_string(), Json::I64(self.code));
        error_object.insert("message".to_string(), Json::String(self.message.to_string()));
        match self.data {            
            Some(ref v) => { error_object.insert("data".to_string(), v.clone()); } ,
            None => (),
        }        
        Json::Object(error_object)
    }
}

pub struct Server {
    methods: BTreeMap<String, Arc<Box<Fn(Json) -> Result<Json,Error> + 'static + Send + Sync >>>
}

impl Server {
    pub fn new() -> Server {
        Server {
            methods : BTreeMap::new()
        }
    }

    pub fn register_method<F>(&mut self, method:&str, f:F) where F: Fn(Json) -> Result<Json,Error> + 'static + Send + Sync  {
        self.methods.insert(method.to_string(), Arc::new(Box::new(f)));
    }

    pub fn request<F>(&self, str_request:String, f_response:F) where F: FnOnce(String) + Send + 'static {
        let data = match Json::from_str(&str_request) {
            Ok(o) => o,
            Err(_) => return f_response(Server::response_error(-32700))
        };
        let obj = match data.as_object() {
            Some(s) => s,
            None => return f_response(Server::response_error(-32600))
        };
        match obj.get("jsonrpc") {
            Some(o) => match o.as_string() {
                Some(s) => if s!="2.0" { return f_response(Server::response_error(-32600)) },
                None => return f_response(Server::response_error(-32600))                
            },
            None => return f_response(Server::response_error(-32600))
        };
        let str_method = match obj.get("method") {
            Some(o) => match o.as_string() {
                Some(s) => s,
                None => return f_response(Server::response_error(-32600))                
            },
            None => return f_response(Server::response_error(-32600))
        };
        let params = match obj.get("params") {
            Some(o) => match *o {
                Json::Array(ref v) => Json::Array(v.clone()),
                Json::Object(ref v) => Json::Object(v.clone()),
                _ => return f_response(Server::response_error(-32600))
            },
            None => Json::Null
        };
        let id:Option<Json> = match obj.get("id") {
            Some(o) => match *o {
                Json::String(ref v) => Some(Json::String(v.clone())),
                Json::I64(ref v) => Some(Json::I64(v.clone())),
                Json::U64(ref v) => Some(Json::U64(v.clone())),
                Json::F64(ref v) => Some(Json::F64(v.clone())),
                Json::Null => Some(Json::Null),
                _ => return f_response(Server::response_error(-32600))
            },
            None => None
        };
        let f = match self.methods.get(str_method) {
            Some(o) => o.clone(),
            None => return f_response(Server::response_error(-32601))
        };
        Deferred::new(move ||{                                        
            f(params)
        }).finally(move |res| {
            if id.is_some() {
                let mut resp_object = BTreeMap::new();
                resp_object.insert("jsonrpc".to_string(), Json::String("2.0".to_string()));
                resp_object.insert("id".to_string(), id.unwrap());                
                match res {
                    Ok(v) => { resp_object.insert("result".to_string(), v); } ,
                    Err(e) => { resp_object.insert("error".to_string(), e.as_object()); }
                } 
                f_response(Json::Object(resp_object).to_string());
            }
        });
    }

    fn response_error(code:i64) -> String {
        let mut resp_object = BTreeMap::new();
        resp_object.insert("jsonrpc".to_string(), Json::String("2.0".to_string()));        
        resp_object.insert("error".to_string(), Error::predefined(code, None).as_object());
        resp_object.insert("id".to_string(), Json::Null);      
        Json::Object(resp_object).to_string()
    }

}

#[macro_export]
macro_rules! rpc_method {
    ( $rpc_struct:expr, $rpc_method:expr, $($n:ident<$t:ty>);+ , $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |json_params| { 
            #[derive(Debug)]
            struct Params { $( $n:$t, ) + }            
            impl $crate::serialize::Decodable for Params {
                fn decode<D: $crate::serialize::Decoder>(d: &mut D) -> ::std::result::Result<Params, D::Error> {
                    d.read_struct("Params", 0usize, |_d| -> _ {
                        ::std::result::Result::Ok(Params{
                            $(
                                $n: match _d.read_struct_field(stringify!($n), 0usize, $crate::serialize::Decodable::decode) {
                                    ::std::result::Result::Ok(v) => v,
                                    ::std::result::Result::Err(v) => return ::std::result::Result::Err(v),
                                },
                            ) +
                        }) 
                    })
                }
            }           
            let mut decoder = $crate::serialize::json::Decoder::new(json_params);
            let rpc_params:Params = match $crate::serialize::Decodable::decode(&mut decoder) {
                Ok(p) => p,
                Err(_) => return Err(Error::predefined(-32602, None))
            };
            $( let $n:$t = rpc_params.$n; ) +                                    
            
            $rpc_block        
        })        
    };        
    ( $rpc_struct:expr, $rpc_method:expr, $n:ident[$t:ty], $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |json_params| {                                     
            let mut $n:Vec<$t> = Vec::new();
            match json_params {
                $crate::serialize::json::Json::Array(a) => {
                    for v in a {
                        let mut decoder = $crate::serialize::json::Decoder::new(v);
                        let val:$t = match $crate::serialize::Decodable::decode(&mut decoder) {                        
                            Ok(p) => p,
                            Err(_) => return Err(Error::predefined(-32602, None))
                        };
                        $n.push(val);
                    }
                },
                _ => return Err(Error::predefined(-32602, None))
            }
            $rpc_block        
        })     
    };                 
    ( $rpc_struct:expr, $rpc_method:expr, $n:ident, $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |json_params| {                         
            let $n:Json = json_params;
            $rpc_block        
        })     
    };         
}

#[macro_export]
macro_rules! rpc_method_no_params {
    ( $rpc_struct:expr, $rpc_method:expr, $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |_| {                         
            $rpc_block        
        })     
    };             
}

#[macro_export]
macro_rules! rpc_params {
    ( $p:expr, $($n:ident<$t:ty>);+ ) => {
        {        
            #[derive(Debug)]
            struct Params { $( $n:$t, ) + }            
            impl $crate::serialize::Decodable for Params {
                fn decode<D: $crate::serialize::Decoder>(d: &mut D) -> ::std::result::Result<Params, D::Error> {
                    d.read_struct("Params", 0usize, |_d| -> _ {
                        ::std::result::Result::Ok(Params{
                            $(
                                $n: match _d.read_struct_field(stringify!($n), 0usize, $crate::serialize::Decodable::decode) {
                                    ::std::result::Result::Ok(v) => v,
                                    ::std::result::Result::Err(v) => return ::std::result::Result::Err(v),
                                },
                            ) +
                        }) 
                    })
                }
            }            
            let mut decoder = $crate::serialize::json::Decoder::new($p);
            let rpc_params:Params = match $crate::serialize::Decodable::decode(&mut decoder) {                
                Ok(p) => p,
                Err(_) => return Err(Error::predefined(-32602, None))
            };
            rpc_params
        }
    };
}

#[cfg(test)]
mod test {
    use super::{Server,Error,Json};
    use super::serialize::json::ToJson;
    use std::collections::BTreeMap;
    use std::thread;    

    #[test]
    fn test_method_by_name() {
        let mut rpc_server = Server::new();
        rpc_method!(rpc_server, Subtract, oper1<u64>;oper2<u64>, {                
            Ok(Json::U64(oper1 - oper2))        
        });
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Subtract\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":1234}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_u64().unwrap(), 1234);
            assert_eq!(obj.get("result").unwrap().as_u64().unwrap(), 19);
        });
        thread::sleep_ms(300);
    }

    #[test]
    fn test_method_by_position() {
        let mut rpc_server = Server::new();
        rpc_method!(rpc_server, Multiply, values[u64], {        
            let mut r = 1;
            for v in values { r *= v }
            Ok(Json::U64(r))
        });  
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Multiply\", \"params\":[5, 6, 7], \"id\":\"SEQ456\"}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_string().unwrap(), "SEQ456");
            assert_eq!(obj.get("result").unwrap().as_u64().unwrap(), 210);
        });
        thread::sleep_ms(300);
    }    

    #[test]
    fn test_manual_register() {
        let mut rpc_server = Server::new();
        rpc_server.register_method("Add", |json_params| {   // json_params: String
            // It uses a macro for parse the String into a Struct. rpc_params : { oper1:u64, oper2:u64 }
            let rpc_params = rpc_params!(json_params, oper1<u64>;oper2<u64> );
            assert_eq!(rpc_params.oper1, 23u64);
            assert_eq!(rpc_params.oper2, 4u64);
            let result = Json::U64(rpc_params.oper1 + rpc_params.oper2);
            Ok(result)            
        });        
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Add\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":-1.4788}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_f64().unwrap(), -1.4788f64);
            assert_eq!(obj.get("result").unwrap().as_u64().unwrap(), 27);
        });
        thread::sleep_ms(300);
    }

    #[test]
    fn test_method_returns_array() {
        let mut rpc_server = Server::new();
        rpc_method!(rpc_server, Sequence, start<u64>;step<f64>;iterations<u64>, {                
            let mut value = start as f64;
            let mut res = Vec::new();
            for _ in 0..iterations { 
                res.push(Json::F64(value));
                value += step;
            }
            Ok(Json::Array(res))        
        });
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Sequence\", \"params\":{\"start\":7, \"step\":0.33, \"iterations\":4}, \"id\":1234}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_u64().unwrap(), 1234);            
            let arr = obj.get("result").unwrap().as_array().unwrap();
            assert_eq!(arr.len(), 4);
            assert_eq!((arr[0].as_f64().unwrap() * 100f64).round(), 700f64);            
            assert_eq!((arr[1].as_f64().unwrap() * 100f64).round(), 733f64);            
            assert_eq!((arr[2].as_f64().unwrap() * 100f64).round(), 766f64);            
            assert_eq!((arr[3].as_f64().unwrap() * 100f64).round(), 799f64);            
        });
        thread::sleep_ms(300);
    }

    #[test]
    fn test_method_returns_object() {
        struct Info {
            amount: u32,
            price: f64,
            description: String,
        }            
        impl ToJson for Info {
            fn to_json(&self) -> Json {
                let mut d = BTreeMap::new();
                d.insert("amount".to_string(), self.amount.to_json());
                d.insert("price".to_string(), self.price.to_json());
                d.insert("description".to_string(), self.description.to_json());
                Json::Object(d)
            }
        }

        let mut rpc_server = Server::new();
        rpc_method_no_params!(rpc_server, GetInfo, {                            
            let info = Info { amount : 15, price: 2.33, description: "Apples".to_string() };
            Ok(info.to_json())        
        });
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"GetInfo\", \"id\":1234}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_u64().unwrap(), 1234);            
            let ret = obj.get("result").unwrap().as_object().unwrap();
            assert_eq!(ret.get("amount").unwrap().as_u64().unwrap(), 15);
            assert_eq!((ret.get("price").unwrap().as_f64().unwrap() * 100f64).round(), 233f64);
            assert_eq!(ret.get("description").unwrap().as_string().unwrap(), "Apples");
        });
        thread::sleep_ms(300);
    }


    #[test]
    fn test_custom_error() {
        let mut rpc_server = Server::new();
        rpc_method!(rpc_server, Division, oper1<f64>;oper2<f64>, {                
            if oper2 == 0f64 {
                Err(Error::custom(784, "Division by zero", Some(Json::F64(oper1))))
            } else {
                Ok(Json::F64(oper1 / oper2))
            }        
        }); 
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Division\", \"params\":{\"oper1\":23, \"oper2\":0}, \"id\":4}".to_string();
        rpc_server.request(str_request, |str_response| {    
            let data = Json::from_str(&str_response).unwrap();     
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_u64().unwrap(), 4);
            assert!(obj.get("error").unwrap().is_object());
            let err = obj.get("error").unwrap().as_object().unwrap();
            assert_eq!(err.get("code").unwrap().as_i64().unwrap(), 784);
            assert_eq!(err.get("message").unwrap().as_string().unwrap(), "Division by zero");
            assert_eq!(err.get("data").unwrap().as_f64().unwrap(), 23f64);
        });
        thread::sleep_ms(300);
    }

    #[test]
    fn test_error_method_not_found() {
        let mut rpc_server = Server::new();
        rpc_method!(rpc_server, Subtract, oper1<u64>;oper2<u64>, {                
            Ok(Json::U64(oper1 - oper2))        
        });
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Add\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":1234}".to_string();
        rpc_server.request(str_request, |str_response| {            
            let data = Json::from_str(&str_response).unwrap();            
            assert!(data.is_object());
            let obj = data.as_object().unwrap();
            assert_eq!(obj.get("jsonrpc").unwrap().as_string().unwrap(), "2.0");
            assert_eq!(obj.get("id").unwrap().as_null().unwrap(), ());
            assert!(obj.get("error").unwrap().is_object());
            let err = obj.get("error").unwrap().as_object().unwrap();
            assert_eq!(err.get("code").unwrap().as_i64().unwrap(), -32601);
            assert_eq!(err.get("message").unwrap().as_string().unwrap(), "Method not found");
        });
        thread::sleep_ms(300);
    }    
}