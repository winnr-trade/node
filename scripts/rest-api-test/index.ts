import {
  adminAddress,
  chainState,
  market,
  marketMakerAddress,
  marketMakerSigner,
  rollup,
  tokenDeployerAddress,
  userAddresses,
  userSigners,
} from "./config";
import {
  createToken,
  getTokenId,
  getTokenMetadata,
  mintTokens,
  parseUnit,
} from "./apis/bank";
import {
  createMarket,
  mintShares,
  setSupportedCollateralToken,
} from "./apis/market";
import { placeOrder } from "./apis/orderbook";
import { testUsd } from "../../test-data/token/data.json";
import testMarketData from "../../test-data/market/data.json";

type OrderParams = Parameters<typeof placeOrder>[0];
type OrderSigner = Parameters<typeof placeOrder>[1];
type Actor = {
  label: string;
  address: string;
  signer: OrderSigner;
};

type LiquidityBookEntry = {
  actor: Actor;
  orders: OrderParams[];
};

type LiquidityPlan = {
  entries: LiquidityBookEntry[];
  totalOrders: number;
  makerAskYes: number;
  makerAskNo: number;
};

type LiquidityConfig = {
  midPrice: number;
  halfSpread: number;
  levels: number;
  makerAsksPerLevel: number;
  makerBidsPerLevel: number;
  userBidsPerLevel: number;
  baseAskQty: number;
  baseBidQty: number;
  levelQtyStep: number;
  clipJitter: number;
};

const EXPECTED_USER_COUNT = 10;

const users: Actor[] = userAddresses.map((address, index) => ({
  label: `user-${index + 1}`,
  address,
  signer: userSigners[index]!,
}));

const marketMaker: Actor = {
  label: "market-maker",
  address: marketMakerAddress,
  signer: marketMakerSigner,
};

if (users.length < EXPECTED_USER_COUNT) {
  throw new Error(
    `Expected ${EXPECTED_USER_COUNT} seeded users, found ${users.length}. Check test-data/keys/users.json.`,
  );
}

const getUser = (index: number): Actor => {
  const user = users[index];
  if (!user) {
    throw new Error(`Missing seeded user at index ${index}`);
  }

  return user;
};

const clampPrice = (price: number): number => Math.max(1, Math.min(99, price));

const LIQUIDITY_CONFIG: LiquidityConfig = {
  midPrice: 50,
  halfSpread: 1,
  levels: 22,
  makerAsksPerLevel: 3,
  makerBidsPerLevel: 1,
  userBidsPerLevel: 4,
  baseAskQty: 110,
  baseBidQty: 85,
  levelQtyStep: 8,
  clipJitter: 19,
};

const quantityFor = (
  baseQty: number,
  level: number,
  clip: number,
  seed: number,
  levelStep: number,
  jitter: number,
): number => {
  const jitterValue = (clip * 7 + seed * 13 + level * 3) % jitter;
  return Math.max(1, baseQty + level * levelStep + jitterValue);
};

const pushOrder = (
  books: Map<string, LiquidityBookEntry>,
  actor: Actor,
  order: OrderParams,
) => {
  const existing = books.get(actor.label);
  if (existing) {
    existing.orders.push(order);
    return;
  }

  books.set(actor.label, { actor, orders: [order] });
};

const buildLiquidityPlan = (
  marketId: number,
  config: LiquidityConfig,
): LiquidityPlan => {
  const books = new Map<string, LiquidityBookEntry>();
  let makerAskYes = 0;
  let makerAskNo = 0;

  const outcomes: Array<"yes" | "no"> = ["yes", "no"];

  for (const [outcomeIndex, outcome] of outcomes.entries()) {
    for (let level = 0; level < config.levels; level += 1) {
      const askPrice = clampPrice(config.midPrice + config.halfSpread + level);
      const bidPrice = clampPrice(config.midPrice - config.halfSpread - level);

      // Market maker provides the visible top-of-book and deep ask stack.
      for (let clip = 0; clip < config.makerAsksPerLevel; clip += 1) {
        const qty = quantityFor(
          config.baseAskQty,
          level,
          clip,
          outcomeIndex,
          config.levelQtyStep,
          config.clipJitter,
        );

        pushOrder(books, marketMaker, {
          marketId,
          outcome,
          side: "ask",
          price: askPrice,
          quantity: qty,
          orderType: "limit",
        });

        if (outcome === "yes") {
          makerAskYes += qty;
        } else {
          makerAskNo += qty;
        }
      }

      // Add some maker bid support so both sides stay deep and spreads remain tight.
      for (let clip = 0; clip < config.makerBidsPerLevel; clip += 1) {
        const qty = quantityFor(
          Math.floor(config.baseBidQty * 0.8),
          level,
          clip,
          outcomeIndex + 5,
          Math.max(1, Math.floor(config.levelQtyStep * 0.7)),
          config.clipJitter,
        );

        pushOrder(books, marketMaker, {
          marketId,
          outcome,
          side: "bid",
          price: bidPrice,
          quantity: qty,
          orderType: "limit",
        });
      }

      // Distribute many bid clips across users for exchange-like depth.
      for (let clip = 0; clip < config.userBidsPerLevel; clip += 1) {
        const userIndex =
          (level * config.userBidsPerLevel + clip + outcomeIndex) %
          users.length;
        const actor = getUser(userIndex);
        const qty = quantityFor(
          config.baseBidQty,
          level,
          clip,
          userIndex,
          config.levelQtyStep,
          config.clipJitter,
        );

        pushOrder(books, actor, {
          marketId,
          outcome,
          side: "bid",
          price: bidPrice,
          quantity: qty,
          orderType: "limit",
        });
      }
    }
  }

  const entries = [...books.values()].filter(
    (entry) => entry.orders.length > 0,
  );
  const totalOrders = entries.reduce(
    (sum, entry) => sum + entry.orders.length,
    0,
  );

  return {
    entries,
    totalOrders,
    makerAskYes,
    makerAskNo,
  };
};

const placeOrdersForActor = async (
  actor: Pick<Actor, "label" | "signer">,
  orders: OrderParams[],
) => {
  for (const [index, order] of orders.entries()) {
    await placeOrder(order, actor.signer).catch((error) => {
      console.error(
        `[${actor.label}] failed placing order #${index + 1}: ${JSON.stringify(order)}`,
      );
      throw error;
    });
  }
};

const placeProgrammaticLiquidity = async (
  plan: LiquidityPlan,
  marketId: number,
) => {
  console.log(
    `Placing ${plan.totalOrders} limit orders on market ${marketId} across ${plan.entries.length} participants...`,
  );

  for (const entry of plan.entries) {
    await placeOrdersForActor(entry.actor, entry.orders);
  }
};

const main = async () => {
  console.log("Rollup chain id:", rollup.context.defaultTxDetails.chain_id);

  // Create a test token
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

  // Set as supported collateral token for markets
  console.log("Setting supported collateral token for markets...");
  await setSupportedCollateralToken({ tokenId, support: true });
  console.log("Collateral token set up.");

  // Mint tokens to market maker
  console.log("Minting tokens to market maker & users...");
  for (const address of [marketMakerAddress, ...userAddresses]) {
    await mintTokens({
      toAddress: address,
      tokenId,
      amount: parseUnit(100000, token.decimals),
    });
  }
  console.log("Minted token.");

  // Create test markets if they don't already exist
  const marketCount = await market.getNextMarketId();
  const currentTime = await chainState.time();
  if (marketCount === 0) {
    console.log("Creating test markets...");
    for (const m of testMarketData.markets) {
      let resolver = m.resolver as any;
      if (m.resolver_type === "address") {
        resolver = { Address: adminAddress };
      } else if (m.resolver_type === "pyth") {
        resolver = {
          Pyth: {
            feed_id: resolver.Pyth.feed_id,
            lower_bound: resolver.Pyth.lower_bound,
            upper_bound: resolver.Pyth.upper_bound,
          },
        };
      } else if (m.resolver_type === "optimistic") {
        resolver = { Optimistic: {} };
      } else {
        throw new Error(`Unknown resolver type: ${m.resolver_type}`);
      }

      await createMarket({
        question: m.question,
        collateralTokenId: tokenId,
        resolutionTime: currentTime + 7 * 24 * 60 * 60 * 1000,
        resolver,
      });
    }
  } else {
    console.log(`Using existing markets (count: ${marketCount})`);
  }

  const marketId = 0;
  console.log(`Provisioning inventory for market ${marketId}...`);

  const liquidityPlan = buildLiquidityPlan(marketId, LIQUIDITY_CONFIG);
  const makerSharesNeeded =
    Math.max(liquidityPlan.makerAskYes, liquidityPlan.makerAskNo) + 500;

  await mintShares(marketId, makerSharesNeeded, marketMakerSigner);

  console.log(
    `Seeding programmatic liquidity (mid=${LIQUIDITY_CONFIG.midPrice}, spread=${LIQUIDITY_CONFIG.halfSpread * 2}, levels=${LIQUIDITY_CONFIG.levels})...`,
  );
  await placeProgrammaticLiquidity(liquidityPlan, marketId);

  console.log(
    "Seed complete: deep and tight orderbook liquidity is now in place.",
  );
};

main()
  .then(() => {
    console.log("Done!");
  })
  .catch((err) => {
    console.error("Error:");
    console.error(JSON.stringify(err, null, 2));
  });
