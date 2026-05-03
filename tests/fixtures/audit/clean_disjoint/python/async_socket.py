import asyncio


async def echo(reader, writer):
    data = await reader.readuntil(b"\n")
    writer.write(data)
    await writer.drain()
    writer.close()
