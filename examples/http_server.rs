#[macro_use(rpc_method, rpc_method_no_params, rpc_params)]
extern crate json_rpc;
extern crate hyper;

use json_rpc::Server as RpcServer;
use json_rpc::{Json, Error};
use json_rpc::serialize::json::ToJson;
use std::io::Read;
use std::io::Write;
use std::collections::BTreeMap;
use hyper::Server as ServerHttp;
use hyper::server::{Request,Response};


fn main() {
    println!("Started server on 10.0.2.15:8080");

    let mut rpc_server = RpcServer::new();

    register_methods(&mut rpc_server);

    ServerHttp::http(move |mut req:Request, mut res:Response| {
        match req.method {
            hyper::Post => {
                let mut str_req = String::new();
                req.read_to_string(&mut str_req).unwrap();
                let mut res = res.start().unwrap();
                match rpc_server.request(str_req) {
                    Some(str_res) => res.write_all(str_res.as_bytes()).unwrap() ,
                    None => (),      
                };
                res.end().unwrap();
            },
            _ => *res.status_mut() = hyper::status::StatusCode::MethodNotAllowed
        }   
    }).listen("10.0.2.15:8080").unwrap();

    println!("Stopped server!");

}


fn register_methods(rpc_server:&mut RpcServer) {
    // Registers a Rpc Method named "Subtract" with two parameter "by Name".
    rpc_method!(rpc_server, Subtract, oper1<u64>;oper2<u64>, {                
        Ok(Json::U64(oper1 - oper2))        
    });
        
    // Registers a Rpc Method named "Multiply" with N parameteres "by Position".
    rpc_method!(rpc_server, Multiply, values[u64], {        
        let mut r = 1;
        for v in values { r *= v }
        Ok(Json::U64(r))
    });    

    // This method can return an error 
    rpc_method!(rpc_server, Division, oper1<f64>;oper2<f64>, {                
        if oper2 == 0f64 {
            Err(Error::custom(1, "Division by zero", Some(Json::F64(oper1))))
        } else {
            Ok(Json::F64(oper1 / oper2))
        }        
    });       

    // Registers a method that returns an Array
    rpc_method!(rpc_server, Sequence, start<u64>;step<f64>;iterations<u64>, {                
        let mut value = start as f64;
        let mut res = Vec::new();
        for _ in 0..iterations { 
            res.push(Json::F64(value));
            value += step;
        }
        Ok(Json::Array(res))        
    });

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

    // Registers a method that returns an Object
    rpc_method_no_params!(rpc_server, GetInfo, {                            
        let info = Info { amount : 15, price: 2.33, description: "Apples".to_string() };
        Ok(info.to_json())        
    });    


    // Register a Rpc manually without macros
    rpc_server.register_method("Add", |json_params| {   // json_params: String
        // It uses a macro for parse the String into a Struct. rpc_params : { oper1:u64, oper2:u64 }
        let rpc_params = rpc_params!(json_params, oper1<u64>;oper2<u64> );
        let result = Json::U64(rpc_params.oper1 + rpc_params.oper2);
        Ok(result)
    });      
}
