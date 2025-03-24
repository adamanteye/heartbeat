# Heartbeat

To show I am still active.

## Example

```sh
curl -X POST \
    -H "Content-Type: application/json" \
    -d '{"source": "linux laptop", "event": "power on", "note": "", "token": "your-secret"}' \
    http://127.0.0.1:9000/
```
