## Testing

To run locally the tests:

```
$ DATABASE_URL="/tmp/wu.db" cargo test --help --no-default-features --features "sqlite" -- --test-threads=1
```
