# Heartbeat

To show I am still active.

## Example

```sh
curl -X POST \
    -H "Authentication Basic: your-secret" \
    -H "Content-Type: application/json" \
    -d '{"source": "linux laptop", "event": "poweron", "note": ""}' \
    http://127.0.0.1:9000/
```
