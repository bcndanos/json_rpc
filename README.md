# json rpc

**[JSON-RPC 2.0 Implementation](http://www.jsonrpc.org/specification) in Rust**

|Crate|Travis|
|:------:|:-------:|
|[![](http://meritbadge.herokuapp.com/json_rpc)](https://crates.io/crates/json_rpc)|[![Build Status](https://travis-ci.org/bcndanos/json_rpc.svg?branch=master)](https://travis-ci.org/bcndanos/json_rpc)|

#Overview

Currently in development. Look at the examples in ./examples for more information.

#License

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
http://opensource.org/licenses/MIT, at your
option. This file may not be copied, modified, or distributed
except according to those terms.

# Examples

This is a basic example with two methods:

```rust
#[macro_use(rpc_method)]
extern crate json_rpc;
use json_rpc::{Server, Json, Error};

fn main() {
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

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Subtract\", \"params\":{\"oper1\":23, \"oper2\":4}, \"id\":2}".to_string();
    match rpc_server.request(str_request) {    
        Some(str_response) => assert_eq!(str_response, "{\"id\":2,\"jsonrpc\":\"2.0\",\"result\":19}") ,
        None => unreachable!(),
    };

    let str_request = "{\"jsonrpc\":\"2.0\",\"method\":\"Multiply\", \"params\":[5, 6, 7], \"id\":3}".to_string();
    match rpc_server.request(str_request) {    
        Some(str_response) => assert_eq!(str_response, "{\"id\":3,\"jsonrpc\":\"2.0\",\"result\":210}") ,
        None => unreachable!(),
    };
}

``` 
