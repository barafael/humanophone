# Quinnipak

This server application collects chord info from publishing peers and broadcasts it to all consuming peers.

It listens for Websocket client connections.
A connected client can identify as a publisher or a consumer.
A publisher may send chords over its websocket connection.
Quinnipak will forward the chord information to each client.
