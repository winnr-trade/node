// import { Keypair } from "@solana/web3.js";
// import bs58 from "bs58";
// const pk = "5087c12ea7c12024b3f798c5d73587463af17c9fce04d9e6fe873893102a6c64";
// const secretKey = Uint8Array.from(Buffer.from(pk, "hex"));
// const kp = Keypair.fromSeed(secretKey);
// const kp = Keypair.generate();
// console.log("kp:", kp.publicKey.toBase58(), kp.secretKey.toHex().slice(0, 64));

import {
  adminAddress,
  chainState,
  market,
  rollup,
  tokenDeployerAddress,
} from "./config";
import { createToken, getTokenId, getTokenMetadata } from "./apis/bank";
import {
  createMarket,
  mintShares,
  setSupportedCollateralToken,
} from "./apis/market";
import { placeOrder } from "./apis/orderbook";
import { testUsd } from "../../test-data/token/data.json";
import testMarketData from "../../test-data/market/data.json";

const main = async () => {
  console.log("Rollup context:", rollup.context);

  const tokenId = getTokenId({
    deployer: tokenDeployerAddress,
    name: testUsd.name,
    decimals: testUsd.decimals,
  });
  let token = await getTokenMetadata(tokenId);
  if (!token) {
    console.log("Token not found, creating test token...");
    token = await createToken({
      name: testUsd.name,
      decimals: testUsd.decimals,
      initialBalance: parseInt(testUsd.initialBalance),
      supplyCap: parseInt(testUsd.supplyCap),
    });
    console.log("Test token created:", tokenId);
  } else {
    console.log("Test token already exists:", tokenId);
  }

  await setSupportedCollateralToken({ tokenId, support: true });

  const marketCount = await market.getNextMarketId();
  const currentTime = await chainState.time();

  if (marketCount > 0) {
    console.log(`Test markets already exist (count: ${marketCount})`);
    return;
  }

  console.log("Creating test markets...");
  for (const m of testMarketData.markets) {
    await createMarket({
      question: m.question,
      collateralTokenId: tokenId,
      resolutionTime: currentTime + 864_000, // 24 hours from now
      resolver: adminAddress,
    });
  }
  const marketId = 0;
  console.log(`Providing liquidity to market...`);
  await mintShares(marketId, 1000);

  console.log("Placing test orders...");

  // Place asks (sellers) at various price levels
  for (let price = 60; price <= 100; price += 10) {
    await placeOrder({
      marketId,
      outcome: "yes",
      price,
      quantity: 100,
      side: "ask",
      orderType: "limit",
    });
    await placeOrder({
      marketId,
      outcome: "no",
      price,
      quantity: 100,
      side: "ask",
      orderType: "limit",
    });
  }

  // Place bids (buyers) at various price levels
  for (let price = 40; price <= 80; price += 10) {
    await placeOrder({
      marketId,
      outcome: "yes",
      price,
      quantity: 100,
      side: "bid",
      orderType: "limit",
    });
    await placeOrder({
      marketId,
      outcome: "no",
      price,
      quantity: 100,
      side: "bid",
      orderType: "limit",
    });
  }

  // Optionally, place some market orders to create fills
  // await placeOrder(marketId, "yes", 100, 50, "bid", "market");
  // await placeOrder(marketId, "no", 100, 50, "bid", "market");
};

main()
  .then(() => {
    console.log("Done!");
  })
  .catch((err) => {
    console.error("Error:");
    console.error(JSON.stringify(err, null, 2));
  });
