* Ping Pong Scheme
  - Each client pings the server (5s)
  - On Ping, server Pongs
  - On Pong timeout, client saves state (Faction preference) and degrades to periodical reconnection attempts
  - On Ping timeout, server cuts connection
* Remove ID from clients, not needed
* Instrumentation and Telemetry
* MIDI improvements: no more dropped events, reliable controller support
* Test on Raspberry Pi
* Internet for Raspberry Pi
* Test with several people
* Keep making sure the server handles edge cases sensibly
* Start server on host start
* Finish horeau
* Make a simple JS-Based web frontend