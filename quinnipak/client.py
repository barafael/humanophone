import asyncio
import pathlib
import ssl

from websockets.client import connect

ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
localhost_pem = pathlib.Path("certs/localhost.crt")
ssl_context.load_verify_locations(localhost_pem)


async def main():
    async with connect("wss://localhost:8000", ssl=ssl_context) as websocket:
        message = await websocket.recv()
        print(f"Received: {message}")


asyncio.run(main())
