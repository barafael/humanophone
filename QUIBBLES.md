* midir cannot detect midi device disconnection
* kord has wonky playback that screeches and crackles badly
* kord chord detection assigns for example ionian mode to C E F# B
* kord playbackhandle is not `Sync`, so it basically cannot be used with tokio tasks. Can't use even `&dyn Playable`, is not `Sync`.
* tokio-websockets does not implement `Stream` for websocket stream for some reason. This makes it cumbersome to test and have nice generic APIs
* No unit tests easily done
