#[macro_use(rpc_method, rpc_method_no_params, rpc_params)]
extern crate json_rpc;

use json_rpc::{Server, Json, Error};
use json_rpc::serialize::json::ToJson;
use std::thread;
use std::collections::BTreeMap;

fn main() {
    println!("Running Example ...");

    let mut rpc_server = Server::new(); 
    
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
        println!("Rpc Params en add: {:?}", rpc_params);
        thread::sleep_ms(1000);
        let result = Json::U64(rpc_params.oper1 + rpc_params.oper2);
        Ok(result)
    });        

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Add\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":1}".to_string();
    new_request(&rpc_server, str_request);
    
    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Subtract\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":2}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Multiply\", \"params\":[5, 6, 7], \"id\":3}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Multiply\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":33}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Division\", \"params\":{\"oper1\":23, \"oper2\":0}, \"id\":4}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Division\", \"params\":{\"oper1\":30, \"oper2\":7}, \"id\":5}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Sequence\", \"params\":{\"start\":7, \"step\":0.33, \"iterations\":4}, \"id\":1234}".to_string();
    new_request(&rpc_server, str_request);

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"GetInfo\", \"id\":1234}".to_string();    
    new_request(&rpc_server, str_request);

    // This operation is Notification. It doesn't include 'id' and doesn't return anything.
    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Division\", \"params\":{\"oper1\":30, \"oper2\":7}}".to_string();
    new_request(&rpc_server, str_request);    
        
    thread::sleep_ms(2000);    
    println!("End Example");
}

fn new_request(rpc_server:&Server, str_request: String) {
    rpc_server.request(str_request.clone(), move |str_response| {
        println!("Executed: \n   request  = {},\n   response = {}", str_request, str_response);
    });
}