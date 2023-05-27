# humanophone
Broadcast chords among peers and make them sing

## Quinnipak

This server program listens for Websocket client connections.

A connected client can identify as a publisher or a consumer.

A publisher may send chords over its websocket connection.

Quinnipak will forward the chord information to each client.

## Pekisch

This client program connects to a websocket server, then identifies as a publisher.

It also opens a local MIDI device.

Each MIDI event on the local MIDI device is forwarded to the websocket server.

## Pehnt

This client program connects to a websocket server, then identifies as a consumer.

It simply prints all the chord messages it gets from the websocket server.

## Morivar

Morivar defines the datastructures for exchange of chords over a websocket connection.
