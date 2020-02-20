// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { threeActorTest, twoActorTest } from "../lib/actor_test";
import "chai/register-should";
import "../lib/setup_chai";
import { expect, request } from "chai";
import { Actor } from "../lib/actors/actor";
import { createDefaultSwapRequest, sleep } from "../lib/utils";

async function assertNoPeersAvailable(actor: Actor, message: string) {
    const peersResponse = await request(actor.cndHttpApiUrl()).get("/peers");

    expect(peersResponse.status).to.equal(200);
    expect(peersResponse.body.peers, message).to.be.empty;
}

async function assertPeersAvailable(alice: Actor, bob: Actor, message: string) {
    const peersResponse = await request(alice.cndHttpApiUrl()).get("/peers");

    expect(peersResponse.status).to.equal(200);
    expect(peersResponse.body.peers, message).to.containSubset([
        {
            id: await bob.cnd.getPeerId(),
        },
    ]);
}

setTimeout(async function() {
    describe("SWAP request with ip address", () => {
        twoActorTest(
            "[Alice] Should not yet see Bob's peer id in her list of peers",
            async function({ alice }) {
                const res = await request(alice.cndHttpApiUrl()).get("/peers");

                expect(res.status).to.equal(200);
                expect(res.body.peers).to.be.empty;
            }
        );

        threeActorTest(
            "[Alice] Should be able to make a swap request via HTTP api using a random peer id and Bob's ip address",
            async function({ alice, bob, charlie }) {
                await assertNoPeersAvailable(
                    alice,
                    "[Alice] Should not yet see Bob's nor Charlie's peer id in her list of peers"
                );

                // Alice send swap request to Bob
                const swapRequest = await createDefaultSwapRequest(bob);
                await alice.cnd.postSwap({
                    ...swapRequest,
                    peer: {
                        peer_id:
                            "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb", // Random peer id on purpose to see if Bob still appears in GET /swaps using the multiaddress
                        address_hint: await bob.cnd
                            .getPeerListenAddresses()
                            .then(addresses => addresses[0]),
                    },
                });

                await sleep(1000);

                await assertNoPeersAvailable(
                    alice,
                    "[Alice] Should not see any peers because the address did not resolve to the given PeerID"
                );

                await assertNoPeersAvailable(
                    bob,
                    "[Bob] Should not see Alice's PeerID because she dialed to a different PeerID"
                );

                await assertNoPeersAvailable(
                    charlie,
                    "[Charlie] Should not see Alice's PeerID because there was no communication so far"
                );
            }
        );

        threeActorTest(
            "[Alice] Should be able to make a swap request via HTTP api to Charlie using his peer ID and his ip address",
            async function({ alice, bob, charlie }) {
                await assertNoPeersAvailable(
                    alice,
                    "[Alice] Should not yet see Bob's nor Charlie's peer id in her list of peers"
                );

                // Alice send swap request to Bob
                await alice.cnd.postSwap(
                    await createDefaultSwapRequest(charlie)
                );

                await sleep(1000);

                await assertNoPeersAvailable(
                    bob,
                    "[Bob] Should not see any peer ids in his list of peers"
                );

                await assertPeersAvailable(
                    alice,
                    charlie,
                    "[Alice] Should see Charlie's peer id in her list of peers after sending a swap request to him using his ip address"
                );

                await assertPeersAvailable(
                    charlie,
                    alice,
                    "[Charlie] Should see Alice's peer ID in his list of peers after receiving a swap request from Alice"
                );
            }
        );
    });

    run();
}, 0);
