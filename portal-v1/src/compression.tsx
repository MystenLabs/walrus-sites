// Compression and decompression utilities

// The decompression function, in typescript
export async function decompressDataGzip(data: ArrayBuffer) {
    let blob = new Blob([data], { type: "application/gzip" })
    let stream = blob.stream().pipeThrough(new DecompressionStream("deflate"));
    let response = await new Response(stream).arrayBuffer().catch(e => { console.error("DecompressionStream error", e) })
    if (!response) {
        console.debug("Trying GZIP")
        stream = blob.stream().pipeThrough(new DecompressionStream("gzip"));
        response = await new Response(stream).arrayBuffer().catch(e => { console.error("DecompressionStream error", e) })
    }

    if (response) return response
}

// The compression function, which reverses the above
export async function compressDataGzip(data: ArrayBuffer) {
    let blob = new Blob([data])
    let stream = blob.stream().pipeThrough(new CompressionStream("deflate"));
    let response = await new Response(stream).arrayBuffer().catch(e => { console.error("Compression error", e) })
    if (response) return response
}
