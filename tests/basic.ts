import { TestClient } from "./test_client";
import { expect } from "chai";
import { BN } from "bn.js";

describe("perpsDemo", () => {
  let tc = new TestClient();
  let custodyExpected;

  it("init", async () => {
    // init
    await tc.initFixture();
    await tc.init();

    // verify
    let perpetualsExpected = {
      admin: tc.admin.publicKey,
      pools: [],
      transferAuthorityBump: tc.authority.bump,
      perpetualsBump: tc.perpetuals.bump,
    };
    let perpetuals = await tc.program.account.perpetuals.fetch(
      tc.perpetuals.publicKey
    );
    expect(JSON.stringify(perpetuals)).to.equal(
      JSON.stringify(perpetualsExpected)
    );
  });

  it("addPool", async () => {
    // add
    await tc.addPool("test pool");

    // verify
    let pool = await tc.program.account.pool.fetch(tc.pool.publicKey);
    let poolExpected = {
      name: "test pool",
      tokens: [],
      aumUsd: "0",
      bump: tc.pool.bump,
      lpTokenBump: pool.lpTokenBump,
    };
    expect(JSON.stringify(pool)).to.equal(JSON.stringify(poolExpected));
  });

  it("addToken", async () => {
    // add
    let oracleConfig = {
      maxPriceError: new BN(10000),
      maxPriceAgeSec: 60,
      oracleType: { test: {} },
      oracleAccount: tc.custodies[0].oracleAccount,
    };
    let pricingConfig = {
      minInitialLeverage: new BN(1),
      maxLeverage: new BN(100),
    };
    await tc.addToken(tc.custodies[0], oracleConfig, pricingConfig);

    let oracleConfig2 = Object.assign({}, oracleConfig);
    oracleConfig2.oracleAccount = tc.custodies[1].oracleAccount;
    await tc.addToken(tc.custodies[1], oracleConfig2, pricingConfig);

    // verify
    let custody = await tc.program.account.custody.fetch(
      tc.custodies[0].custody
    );
    custodyExpected = {
      tokenAccount: tc.custodies[0].tokenAccount,
      mint: tc.custodies[0].mint.publicKey,
      decimals: 9,
      oracle: {
        oracleAccount: tc.custodies[0].oracleAccount,
        oracleType: { test: {} },
        maxPriceError: "10000",
        maxPriceAgeSec: 60,
      },
      pricing: {
        minInitialLeverage: "1",
        maxLeverage: "100",
      },
      assets: { collateral: "0", owned: "0", locked: "0" },
      bump: custody.bump,
      tokenAccountBump: custody.tokenAccountBump,
    };
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));
  });

  it("setTestOraclePrice", async () => {
    // set
    await tc.setTestOraclePrice(tc.toTokenAmount(1.23, 3), tc.custodies[0]);
    await tc.setTestOraclePrice(tc.toTokenAmount(2.0, 3), tc.custodies[1]);

    // verify
    let oracle = await tc.program.account.testOracle.fetch(
      tc.custodies[0].oracleAccount
    );
    let oracleExpected = {
      price: new BN(1230),
      expo: -3,
      conf: new BN(0),
      publishTime: oracle.publishTime,
    };
    expect(JSON.stringify(oracle)).to.equal(JSON.stringify(oracleExpected));
  });

  it("addLiquidity", async () => {
    // add
    await tc.addLiquidity(
      tc.toTokenAmount(10, tc.custodies[0].decimals),
      tc.user,
      tc.user.tokenAccounts[0],
      tc.custodies[0]
    );

    await tc.addLiquidity(
      tc.toTokenAmount(10, tc.custodies[1].decimals),
      tc.user,
      tc.user.tokenAccounts[1],
      tc.custodies[1]
    );

    // verify
    let custody = await tc.program.account.custody.fetch(
      tc.custodies[0].custody
    );
    custodyExpected.assets.owned = "10000000000";
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));
  });

  it("swap", async () => {
    // swap
    let initialBalance0 = await tc.getBalance(tc.user.tokenAccounts[0]);
    let initialBalance1 = await tc.getBalance(tc.user.tokenAccounts[1]);

    await tc.swap(
      tc.toTokenAmount(5, tc.custodies[0].decimals),
      tc.toTokenAmount(1, tc.custodies[1].decimals),
      tc.user,
      tc.user.tokenAccounts[0],
      tc.user.tokenAccounts[1],
      tc.custodies[0],
      tc.custodies[1]
    );

    // verify
    let custody = await tc.program.account.custody.fetch(
      tc.custodies[0].custody
    );
    custodyExpected.assets.owned = "15000000000";
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));

    let balance0 = await tc.getBalance(tc.user.tokenAccounts[0]);
    let balance1 = await tc.getBalance(tc.user.tokenAccounts[1]);

    expect(initialBalance0 - balance0).to.equal(5000000000);
    expect(balance1 - initialBalance1).to.equal(3075000);
  });

  it("removeLiquidity", async () => {
    // remove
    await tc.removeLiquidity(
      tc.toTokenAmount(10, 6),
      tc.user,
      tc.user.tokenAccounts[0],
      tc.custodies[0]
    );

    await tc.removeLiquidity(
      tc.toTokenAmount(1, 6),
      tc.user,
      tc.user.tokenAccounts[1],
      tc.custodies[1]
    );

    // verify
    let custody = await tc.program.account.custody.fetch(
      tc.custodies[0].custody
    );
    custodyExpected.assets.owned = "6869918700";
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));
  });

  it("openPosition", async () => {
    await tc.openPosition(
      tc.toTokenAmount(1.23, 3),
      tc.toTokenAmount(1, tc.custodies[0].decimals),
      tc.toTokenAmount(10, tc.custodies[0].decimals),
      "long",
      tc.user,
      tc.user.tokenAccounts[0],
      tc.user.positionAccountsLong[0],
      tc.custodies[0]
    );
  });

  it("closePosition", async () => {
    await tc.closePosition(
      tc.toTokenAmount(1.23, 3),
      tc.toTokenAmount(5, tc.custodies[0].decimals),
      tc.user,
      tc.user.tokenAccounts[0],
      tc.user.positionAccountsLong[0],
      tc.custodies[0]
    );
  });

  it("liquidate", async () => {
    await tc.liquidate(
      tc.user,
      tc.user.tokenAccounts[0],
      tc.user.tokenAccounts[0],
      tc.user.positionAccountsLong[0],
      tc.custodies[0]
    );
  });
});
