extern crate asynchronous;
extern crate rustc_serialize;

use asynchronous::Deferred;
use std::collections::BTreeMap;
use std::sync::Arc;
pub use rustc_serialize::json::{self,Json};

pub struct Rpc {
    methods: 
        BTreeMap<String, Arc<Box<Fn(Json) -> Result<Json,String> + 'static + Send + Sync >>>
}
impl Rpc {
    pub fn new() -> Rpc {
        Rpc {
            methods : BTreeMap::new()
        }
    }

    pub fn register_method<F>(&mut self, method:&str, f:F) where F: Fn(Json) -> Result<Json,String> + 'static + Send + Sync  {
        self.methods.insert(method.to_string(), Arc::new(Box::new(f)));
    }

    pub fn request<F>(&self, str_request:String, f_response:F) where F: FnOnce(String) + Send + 'static {
        let data = match Json::from_str(&str_request) {
            Ok(o) => o,
            Err(_) => return f_response(Rpc::response_error(-32700,"Parse error"))            
        };
        let obj = match data.as_object() {
            Some(s) => s,
            None => return f_response(Rpc::response_error(-32600, "Invalid Request"))
        };
        match obj.get("jsonrpc") {
            Some(o) => match o.as_string() {
                Some(s) => if s!="2.0" { return f_response(Rpc::response_error(-32600, "Invalid Request")) },
                None => return f_response(Rpc::response_error(-32600, "Invalid Request"))                
            },
            None => return f_response(Rpc::response_error(-32600, "Invalid Request"))
        };
        let str_method = match obj.get("method") {
            Some(o) => match o.as_string() {
                Some(s) => s,
                None => return f_response(Rpc::response_error(-32600, "Invalid Request"))                
            },
            None => return f_response(Rpc::response_error(-32600, "Invalid Request"))
        };
        let params = match obj.get("params") {
            Some(o) => match *o {
                Json::Array(ref v) => Json::Array(v.clone()),
                Json::Object(ref v) => Json::Object(v.clone()),
                _ => return f_response(Rpc::response_error(-32600, "Invalid Request"))
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
                _ => return f_response(Rpc::response_error(-32600, "Invalid Request"))
            },
            None => None
        };
        let f = match self.methods.get(str_method) {
            Some(o) => o.clone(),
            None => return f_response(Rpc::response_error(-32601, "Method not found"))
        };
        Deferred::new(move ||{                                        
            f(params)
        }).finally(move |res| {
            if id.is_some() {
                let mut resp_object = BTreeMap::new();
                resp_object.insert("jsonrpc".to_string(), Json::String("2.0".to_string()));
                resp_object.insert("id".to_string(), id.unwrap());                
                match res {
                    Ok(r) => {
                        resp_object.insert("result".to_string(), r);
                    },
                    Err(e) => {
                        let mut error_object = BTreeMap::new();
                        error_object.insert("code".to_string(), Json::U64(5));
                        error_object.insert("message".to_string(), Json::String(e));
                        resp_object.insert("error".to_string(), Json::Object(error_object));
                    }
                } 
                f_response(Json::Object(resp_object).to_string());
            }
        });
    }

    fn response_error(code:i64, message:&str) -> String {
        let mut resp_object = BTreeMap::new();
        resp_object.insert("jsonrpc".to_string(), Json::String("2.0".to_string()));
        let mut error_object = BTreeMap::new();
        error_object.insert("code".to_string(), Json::I64(code));
        error_object.insert("message".to_string(), Json::String(message.to_string()));
        resp_object.insert("error".to_string(), Json::Object(error_object));
        resp_object.insert("id".to_string(), Json::Null);      
        Json::Object(resp_object).to_string()
    }

}

macro_rules! rpc_method {
    ( $rpc_struct:expr, $rpc_method:expr, $($n:ident<$t:ty>);+ , $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |json_params| { 
            #[derive(RustcDecodable, Debug)]
            struct Params { $( $n:$t, ) + }            
            let rpc_params:Params = json::decode(&json_params.to_string()).unwrap(); // TODO: Validate format
            $( let $n:$t = rpc_params.$n; ) +                                    
            
            $rpc_block        
        })        
    };        
    ( $rpc_struct:expr, $rpc_method:expr, $n:ident[$t:ty], $rpc_block:block ) => {
        $rpc_struct.register_method(stringify!($rpc_method), |json_params| {                                     
            let mut $n:Vec<$t> = Vec::new();
            match json_params {
                Json::Array(a) => {
                    for v in a {
                        let val:$t = json::decode(&v.to_string()).unwrap();
                        $n.push(val);
                    }
                },
                _ => panic!(),  // TODO: Check error
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

macro_rules! rpc_params {
    ( $p:expr, $($n:ident<$t:ty>);+ ) => {
        {        
            #[derive(RustcDecodable, Debug)]
            struct Params { $( $n:$t, ) + }            
            let rpc_params:Params = json::decode(&$p.to_string()).unwrap();
            rpc_params
        }
    };
}

#[cfg(test)]
mod test {
    use super::Rpc;
    use std::thread;
    use rustc_serialize::json::{self, Json};

    #[test]
    fn test_1() {
        let mut rpc = Rpc::new();
        rpc_method!(rpc, Subtract, oper1<u64>;oper2<u64>, {                
            Ok(Json::U64(oper1 - oper2))        
        });
        let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Add\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":1}".to_string();
        rpc.request(str_request, |str_response| {
            println!("Response Add: {}", str_response);
        });
        thread::sleep_ms(1000);
    }
}