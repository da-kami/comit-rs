import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import * as path from "path";
import { existsAsync, writeFileAsync } from "../utils";
import getPort from "get-port";
import { Logger } from "log4js";
import waitForLogMessage from "../wait_for_log_message";
import { BitcoinNodeConfig, LedgerInstance } from "./index";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";

export class BitcoindInstance implements LedgerInstance {
    private process: ChildProcess;
    private username: string;
    private password: string;

    public static async new(
        dataDir: string,
        pidFile: string,
        logger: Logger
    ): Promise<BitcoindInstance> {
        return new BitcoindInstance(
            dataDir,
            pidFile,
            logger,
            await getPort({ port: 18444 }),
            await getPort({ port: 18443 }),
            await getPort({ port: 28332 }),
            await getPort({ port: 28333 })
        );
    }

    constructor(
        private readonly dataDir: string,
        private readonly pidFile: string,
        private readonly logger: Logger,
        public readonly p2pPort: number,
        public readonly rpcPort: number,
        public readonly zmqPubRawBlockPort: number,
        public readonly zmqPubRawTxPort: number
    ) {}

    public async start() {
        const bin = await this.findBinary();

        this.logger.info("Using binary", bin);

        await this.createConfigFile(this.dataDir);

        this.process = spawn(bin, [`-datadir=${this.dataDir}`], {
            cwd: this.dataDir,
            stdio: "ignore",
        });

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.info(
                "bitcoind exited with code",
                code,
                "after signal",
                signal
            );
        });

        await waitForLogMessage(this.logPath(), "init message: Done loading");

        const result = fs.readFileSync(
            path.join(this.dataDir, "regtest", ".cookie"),
            "utf8"
        );
        const [username, password] = result.split(":");

        this.username = username;
        this.password = password;

        this.logger.info("bitcoind started with PID", this.process.pid);

        await writeFileAsync(this.pidFile, this.process.pid, {
            encoding: "utf-8",
        });
    }

    public get config(): BitcoinNodeConfig {
        return {
            network: "regtest",
            host: "localhost",
            rpcPort: this.rpcPort,
            p2pPort: this.p2pPort,
            username: this.username,
            password: this.password,
            dataDir: this.getDataDir(),
            rpcUrl: `http://localhost:${this.rpcPort}`,
        };
    }

    private async findBinary(): Promise<string> {
        const envOverride = process.env.BITCOIND_BIN;

        if (envOverride) {
            this.logger.info(
                "Overriding bitcoind bin with BITCOIND_BIN: ",
                envOverride
            );

            return envOverride;
        }

        const cacheDir = findCacheDir({
            name: "bitcoin-core-0.17.0",
            create: true,
            thunk: true,
        });
        const binary = cacheDir("bitcoin-0.17.0", "bin", "bitcoind");

        if (await existsAsync(binary)) {
            return binary;
        }

        this.logger.info("Binary not found at ", binary, ", downloading ...");

        await download(BitcoindInstance.downloadUrl(), cacheDir(""), {
            decompress: true,
            extract: true,
        });

        this.logger.info("Download completed");

        return binary;
    }

    private static downloadUrl() {
        switch (platform()) {
            case "darwin":
                return "https://bitcoincore.org/bin/bitcoin-core-0.17.0/bitcoin-0.17.0-osx64.tar.gz";
            case "linux":
                return "https://bitcoincore.org/bin/bitcoin-core-0.17.0/bitcoin-0.17.0-x86_64-linux-gnu.tar.gz";
            default:
                throw new Error(`Unsupported platform ${platform()}`);
        }
    }

    private logPath() {
        return path.join(this.dataDir, "regtest", "debug.log");
    }

    public getDataDir() {
        return this.dataDir;
    }

    private async createConfigFile(dataDir: string) {
        const output = `regtest=1
server=1
printtoconsole=1
rpcallowip=0.0.0.0/0
nodebug=1
rest=1
acceptnonstdtxn=0
zmqpubrawblock=tcp://127.0.0.1:${this.zmqPubRawBlockPort}
zmqpubrawtx=tcp://127.0.0.1:${this.zmqPubRawTxPort}

[regtest]
bind=0.0.0.0:${this.p2pPort}
rpcbind=0.0.0.0:${this.rpcPort}
`;
        const config = path.join(dataDir, "bitcoin.conf");
        await writeFileAsync(config, output);
    }
}
