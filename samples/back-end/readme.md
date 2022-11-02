# moth samples/back-end

## Purpose

A *NodeJs* based HTTP web-server that serves the front-end static code to client, opening also a web-socket server able to answer some basic commands.

## How to run

```
node runner.js "../front-end/html/"
```

## Configuration

By default, the runner opens the following ports:

```
1000	HTTP server port
2000	Web-Socket server port
```

## WebSocket data exchanges

### Connection

When a new socket client connects, the server send the following data:

```json
{
    message: "(Back-End) you're now connected 'socketRemoteAddress'"
}
```

### Front-end to back-end data

#### Informative messages

Receive a simple message to log in the console.

```json
{
    message: "message to log"
}
```

#### Stream (de)activation

Receive an image stream (de)activation order.

```json
{
    stream: true
}
```

### Back-end to front-end data

When the image streaming is active, the back-end sends continuously Base64 encoded JPG image data:

```json
{
    image: "base64JPGimageData"
}
```

