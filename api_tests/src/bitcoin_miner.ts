import { sleep } from "./utils";
import { BitcoinNodeConfig } from "./ledgers";
import BitcoinRpcClient from "bitcoin-core";
import { promises as asyncFs } from "fs";

const configFile = process.argv[2];

// tslint:disable-next-line:no-floating-promises
run(configFile);

async function run(configFile: string) {
    const config: BitcoinNodeConfig = await asyncFs
        .readFile(configFile, {
            encoding: "utf-8",
        })
        .then(JSON.parse);

    const client = new BitcoinRpcClient({
        network: "regtest",
        host: "localhost",
        port: config.rpcPort,
        username: config.username,
        password: config.password,
        wallet: config.minerWallet,
    });

    const blockRewardAddress = await client.getNewAddress();

    // only coins after the first 101 are spendable
    await client.generateToAddress(101, blockRewardAddress);

    while (true) {
        await client.generateToAddress(1, blockRewardAddress);

        await sleep(1000);
    }
}
