# About

This is a small demo produced to support the talk "Dynamic schemas with serde" at Rust Sydney April 2024.

It demonstrates how to leverage `serde` and `serde_json` to handle the serialization and deserialization of JSON documents into a compact binary format (in this project called "flatbin", because naming things is hard).

The binary format is novel in two aspects:

- It is not self-describing, and relies on a schema type (`Ty`) to encode and decode documents. This is for maximum compactness, e.g. object keys do not need to be encoded. This is not unlike other encoding schemes like messagepack or protobuf.
- It is possible to separate out the bytes representing the discrete elements of an array of fields of an object without knowing their types. So, for example, it is possible to skip over the first three elements of an array and extract the fourth one without knowing what kinds of values that array holds.

The unit tests in `src/tests.rs` and benchmarks in `benches/serde.rs` show how to use the library code to encode and decode documents given a runtime schema.

The code in `src/slow` shows how to encode/decode documents using an intermediate `serde_json::Value` object to represent arbitrary JSON, and is a very simple but relatively slower method.

The code in `src/fast` shows how to go directly from JSON text to encoded bytes and back again without this intermediate value, by implementing serde traits such as `Serialize`, `DeserializeSeed` and `Visitor`.
