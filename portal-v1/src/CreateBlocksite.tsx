import { TransactionBlock } from "@mysten/sui.js/transactions";
import { Container, Heading } from "@radix-ui/themes";
import {
    useCurrentAccount,
    useSignAndExecuteTransactionBlock,
    useSuiClient,
} from "@mysten/dapp-kit";
import { useNetworkVariable } from "./networkConfig";
import { SUI_CLOCK_OBJECT_ID } from "@mysten/sui.js/utils";
import { compressDataGzip } from "./compression";
// import { bcs, BCS, getSuiMoveConfig } from "@mysten/bcs";
import { bcs } from "@mysten/bcs";

const MAX_TX_LEN = 131_072;
const MAX_ARG_LEN = 16_380;
const MARGIN = 3; // we need a bit more space in th ebuffer otherwise it fails

export function CreateBlocksite({
    onCreated,
}: {
    onCreated: (id: string) => void;
}) {
    const client = useSuiClient();
    const account = useCurrentAccount();
    const blocksitePackageId = useNetworkVariable("blocksitePackageId");
    const { mutate: signAndExecute } = useSignAndExecuteTransactionBlock();

    return (
        <>
            <Container mt="5">
                <Heading size="3">... or create a new blocksite</Heading>
                <div style={{ marginBottom: "1rem", justifyContent: "center", alignItems: "center", display: "flex" }}>
                    <div
                        onDragOver={(event) => {
                            event.preventDefault();
                        }}
                        onDrop={async (event) => {
                            event.preventDefault();
                            const file = event.dataTransfer.files[0];
                            const reader = new FileReader();
                            reader.onload = async (event) => {
                                const contents = event.target?.result as string;
                                await create(contents);
                            };
                            reader.readAsText(file);
                        }}
                        style={{
                            width: "100%",
                            height: "200px",
                            border: "2px dashed gray",
                            borderRadius: "10px",
                            display: "flex",
                            justifyContent: "center",
                            alignItems: "center",
                            cursor: "pointer",
                        }}
                    >
                        <div>
                            Drag and drop your (max ~130KB gzipped) website file here. <br />
                            For bigger sites, use mutliple transactions.
                        </div>
                    </div>
                </div>
            </Container>
        </>
    );

    // The create function should take the html from the input field
    async function create(data: string) {
        const txb = new TransactionBlock();

        // Convert the data to a Uint8Array
        const dataBytes = new TextEncoder().encode(data);
        // Compress the data, asynchronously
        const compressedData = await compressDataGzip(dataBytes);

        if (!compressedData) {
            console.error("Failed to compress data");
            return;
        }

        let uint8Data = new Uint8Array(compressedData);
        console.log("Total length:", uint8Data.length);
        if (uint8Data.length > MAX_TX_LEN) { return; }

        // Serialize only the first MAX_LEN bytes, otherwise the transaction will fail
        // b/c we are exceeding the maximum argument size
        let firstSlice = uint8Data.slice(0, Math.min(MAX_ARG_LEN, uint8Data.length));
        let serialized = bcs.vector(bcs.u8()).serialize(firstSlice, { size: firstSlice.length + MARGIN });
        let [blocksite] = txb.moveCall({
            arguments: [txb.pure(serialized), txb.object(SUI_CLOCK_OBJECT_ID)],
            target: `${blocksitePackageId}::blocksite::create`,
        });

        // Now add the other pieces if necessary
        if (uint8Data.length > MAX_ARG_LEN) {
            for (let index = MAX_ARG_LEN; index < uint8Data.length; index += MAX_ARG_LEN) {
                let end = Math.min(index + MAX_ARG_LEN, uint8Data.length);
                let element = uint8Data.slice(index, end);
                let serialized = bcs.vector(bcs.u8()).serialize(element, { size: element.length + MARGIN });
                txb.moveCall({
                    arguments: [blocksite, txb.pure(serialized), txb.object(SUI_CLOCK_OBJECT_ID)],
                    target: `${blocksitePackageId}::blocksite::add_piece`
                })
            }
        }

        if (account) {
            txb.transferObjects([blocksite], account.address);
        }

        signAndExecute(
            {
                transactionBlock: txb,
                options: {
                    showEffects: true,
                    showObjectChanges: true,
                },
            },
            {
                onSuccess: async (tx) => {
                    client
                        .waitForTransactionBlock({
                            digest: tx.digest,
                        })
                        .then(async () => {
                            const objectId = tx.effects?.created?.[0]?.reference?.objectId;
                            if (objectId) {
                                console.log("Created blocksite:", objectId);
                                onCreated(objectId);
                            }
                        });
                },
            },
        );
    }
}

