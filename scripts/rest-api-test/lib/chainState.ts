import type { Rollup } from "@sovereign-sdk/web3";

// Adjust these payloads to the actual JSON shape returned by /modules/chain_state/...
type ChainParamsPayload = {
  key: string;
  value: {
    chain_id: string;
    // add other fields you care about
  };
};

type BlockHeaderPayload = {
  height: string;
  hash: string;
  time: string;
  // add other fields you care about
};

export type ChainParams = {
  chainId: string;
  // map other params you expose
};

export type BlockHeader = {
  height: bigint;
  hash: string;
  time: string;
  // map other fields you expose
};

/**
 * ChainState class for interacting with the Sovereign SDK ChainState module.
 */
export class ChainState {
  // biome-ignore lint/suspicious/noExplicitAny: types arent used
  private readonly rollup: Rollup<any, any>;

  private readonly prefix: string = "/modules/chain-state";

  // biome-ignore lint/suspicious/noExplicitAny: types arent used
  constructor(rollup: Rollup<any, any>) {
    this.rollup = rollup;
  }

  async time(): Promise<number> {
    const response: any = await this.rollup.http.get(
      `${this.prefix}/state/time`,
    );

    return response.value;
  }

  /**
   * Gets current chain parameters.
   *
   * @returns Promise resolving to chain parameters
   */
  async params(): Promise<ChainParams> {
    const response: ChainParamsPayload = await this.rollup.http.get(
      `${this.prefix}/state/params`,
    );
    return {
      chainId: response.value.chain_id,
      // map other fields
    };
  }

  /**
   * Gets the latest block header.
   *
   * @returns Promise resolving to the latest block header
   */
  async latestHeader(): Promise<BlockHeader> {
    const response: BlockHeaderPayload = await this.rollup.http.get(
      `${this.prefix}/state/headers/latest`,
    );
    return {
      height: BigInt(response.height),
      hash: response.hash,
      time: response.time,
      // map other fields
    };
  }

  /**
   * Gets a block header by height.
   *
   * @param height - Block height
   * @returns Promise resolving to the block header at the given height
   */
  async headerAtHeight(height: bigint | number): Promise<BlockHeader> {
    const h =
      typeof height === "bigint" ? height.toString() : height.toString();
    const response: BlockHeaderPayload = await this.rollup.http.get(
      `${this.prefix}/state/headers/${h}`,
    );
    return {
      height: BigInt(response.height),
      hash: response.hash,
      time: response.time,
      // map other fields
    };
  }
}
