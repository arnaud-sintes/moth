# *moth* samples/back-end

## Purpose

A *NodeJs* based HTTP web-server that serves the front-end static code to client, opening also a web-socket server able to answer some basic commands.

## Installation

We need to install [NodeJs](https://nodejs.org/en/download/) (including *npm*) to make this sample running well.

The first time, we need to install the "*ws*" and "*express*" packages:

```
npm install ws express
```

## How to run

```
node runner.js "../front-end/html/"
```

We must see this:

```
> Http listening over folder '../front-end/html/' on port 1000
> WebSocket listening on port 2000
```

## Configuration

By default, the runner opens the following ports:

```
1000	HTTP server port
2000	Web-Socket server port
```

## WebSocket JSON data exchanges

### Connection

When a new socket client connects, the server send to the client the following data:

```json
{
    "message": "(Back-End) you're now connected 'socketRemoteAddress'"
}
```

### Front-end to back-end data

#### Informative messages

Receive a simple message to be logged in the console.

```json
{
    "message": "message to log"
}
```

#### Stream (de)activation

Receive an image stream (de)activation order.

```json
{
    "stream": true
}
```

### Back-end to front-end data

When the image streaming is active, the back-end sends continuously *Base64* encoded JPG image data (performance testing):

```json
{
    "image": "base64JPGimageData"
}
```

