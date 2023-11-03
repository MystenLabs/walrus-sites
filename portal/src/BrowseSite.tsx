import {
    useSuiClientQuery,
} from "@mysten/dapp-kit";
import { SuiObjectData } from "@mysten/sui.js/client";
import {Box, Flex, Heading, Link, Separator, Text } from "@radix-ui/themes";
import { decompressDataGzip } from "./compression";
import { useEffect, useState } from "react";

export function BrowseSite({ id }: { id: string }) {
    const { data, isLoading, error, } = useSuiClientQuery("getObject", {
        id,
        options: {
            showContent: true,
            showOwner: true,
        },
    });

    if (isLoading) return <Text>Loading...</Text>;

    if (error) return <Text>Error: {error.message}</Text>;

    if (!data.data) return <Text>Not found</Text>;

    let fields = getCounterFields(data.data);
    let created = fields?.created;
    let dateTs = "No date";
    if (created) {
        let ts = new Date(created * 1);
        dateTs = ts.toLocaleString();
    }

    // Decompress the contents
    let contents = fields?.contents;
    console.log("Contents:", contents);
    
    // Convert into an uitn8array
    const contentsUint8Array = contents ? new Uint8Array(contents) : new Uint8Array();

    // TODO: Programmatically get the network name
    let network = "devnet";
    const explorerLink = `https://suiexplorer.com/object/${id}?network=${network}`;

    function DecompressedContent({ contents }: { contents: Uint8Array | undefined }) {
        const [decompressed, setDecompressed] = useState("");

        useEffect(() => {
            if (contents) {
                decompressContents(contents).then((result) => {
                    setDecompressed(result);
                });
            }
        }, [contents]);

        // Add the <base target="_top"> tag to the head of the decompressed HTML
        // This allows the links to work correctly
        // TODO: ok?
        let el =  document.createElement('html');
        el.innerHTML = decompressed;

        let head = el.getElementsByTagName('head')[0];
        let base = document.createElement('base');
        base.setAttribute('target', '_top');
        head.appendChild(base);
        // Get the blocksite title
        let blocksite_title = el.getElementsByTagName('title')[0]?.innerHTML;
        if (blocksite_title) {
            // Set it inthe outer document
            document.title = blocksite_title;   
        }
    
        // Now return the iframe as html
        return (
            <iframe
                title="Rendered Website"
                // TODO: SECURITY CHECKS
                sandbox="allow-same-origin allow-scripts allow-forms allow-top-navigation allow-popups allow-modals allow-popups-to-escape-sandbox allow-downloads"
                srcDoc={el.outerHTML}
                style={{ width: '100%',  minHeight: "100vh",  flexDirection: "column", border: 'none', flexGrow: "1", display:"flex" }}
            ></iframe>
        );
    }

    return (
        <>
            <Flex style={{position: "absolute", width: "100%", height: "100vh"}}> 
                <DecompressedContent contents={contentsUint8Array} />
            </Flex>
            <Flex justify="end">
            <Flex
                position="sticky"
                px="4"
                py="2"
                style={{
                    marginTop: "10px",
                    position: "sticky", 
                    borderRadius: "10px",
                    boxShadow: "0 0 20px 0 rgba(0,0,0,0.2)",
                    backgroundColor: "white",
                }}
            >
                <Box pr="4">
                    <Link href="/">
                        <Heading>BlockSite</Heading>
                    </Link>
                </Box>
                <Separator orientation="vertical" size="4" />
                <Box pl="4">
                    <Text size="1" align="right" as="div">
                        Version {getCounterFields(data.data)?.version} of {dateTs}
                        <br />
                        <Link target="_blank" href={explorerLink}>View in the Explorer</Link>
                    </Text>
                </Box>
            </Flex>
            </Flex>
        </>
    );
}

function getCounterFields(data: SuiObjectData) {
    if (data.content?.dataType !== "moveObject") {
        return null;
    }
    console.log("Data:", data.content.fields);
    let result = data.content.fields as { created: number, contents: number[]; version: number };
    return result
}

async function decompressContents(data_array: Uint8Array) {
    const arrayBuffer = data_array.buffer;
    console.log("Array buffer:", arrayBuffer);
    const decompressedData = await decompressDataGzip(arrayBuffer);
    console.log("Decompressed data:", decompressedData);
    const decompressedString = new TextDecoder().decode(decompressedData);
    return decompressedString;
}

// Reload the page when the hash changes, s.t. the links to other blocksites work
// TODO: ok?
window.onhashchange = function() {
    window.location.reload()
}