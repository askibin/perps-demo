import * as anchor from "@project-serum/anchor";
import { PerpsDemo } from "../target/types/perps_demo";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  AccountMeta,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { BN } from "bn.js";

export type PositionSide = "long" | "short";

export class TestClient {
  provider: anchor.AnchorProvider;
  program: anchor.Program<PerpsDemo>;
  admin: Keypair;

  // pdas
  authority: { publicKey: PublicKey; bump: number };
  perpetuals: { publicKey: PublicKey; bump: number };
  pool: { publicKey: PublicKey; bump: number };
  lpToken: { publicKey: PublicKey; bump: number };

  custodies: {
    mint: Keypair;
    tokenAccount: PublicKey;
    oracleAccount: PublicKey;
    custody: PublicKey;
    decimals: number;
  }[];
  custodyMetas: AccountMeta[];

  user: {
    wallet: Keypair;
    tokenAccounts: PublicKey[];
    lpTokenAccount: PublicKey;
    positionAccountsLong: PublicKey[];
    positionAccountsShort: PublicKey[];
  };

  constructor() {
    this.provider = anchor.AnchorProvider.env();
    anchor.setProvider(this.provider);
    this.program = anchor.workspace.PerpsDemo as anchor.Program<PerpsDemo>;

    anchor.BN.prototype.toJSON = function () {
      return this.toString(10);
    };
  }

  initFixture = async () => {
    this.admin = Keypair.generate();

    // pdas
    this.authority = await this.findProgramAddress("transfer_authority");
    this.perpetuals = await this.findProgramAddress("perpetuals");
    this.pool = await this.findProgramAddress("pool", "test pool");
    this.lpToken = await this.findProgramAddress("lp_token_mint", [
      this.pool.publicKey,
    ]);

    // custodies
    this.custodies = [];
    this.custodies.push(await this.generateCustody(9));
    this.custodies.push(await this.generateCustody(6));

    this.custodyMetas = [];
    for (const custody of this.custodies) {
      this.custodyMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: custody.custody,
      });
    }
    for (const custody of this.custodies) {
      this.custodyMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: custody.oracleAccount,
      });
    }

    // airdrop funds
    await this.confirmTx(await this.requestAirdrop(this.admin.publicKey));

    // create mints
    for (const custody of this.custodies) {
      await spl.createMint(
        this.provider.connection,
        this.admin,
        this.admin.publicKey,
        null,
        custody.decimals,
        custody.mint
      );
    }

    // user
    let wallet = Keypair.generate();
    await this.requestAirdrop(wallet.publicKey);

    let tokenAccounts = [];
    let positionAccountsLong = [];
    let positionAccountsShort = [];
    for (const custody of this.custodies) {
      let tokenAccount = await spl.createAssociatedTokenAccount(
        this.provider.connection,
        this.admin,
        custody.mint.publicKey,
        wallet.publicKey
      );
      await this.mintTokens(
        1000,
        custody.decimals,
        custody.mint.publicKey,
        tokenAccount
      );
      tokenAccounts.push(tokenAccount);

      let positionAccount = (
        await this.findProgramAddress("position", [
          wallet.publicKey,
          this.pool.publicKey,
          custody.custody,
          [1],
        ])
      ).publicKey;
      positionAccountsLong.push(positionAccount);

      positionAccount = (
        await this.findProgramAddress("position", [
          wallet.publicKey,
          this.pool.publicKey,
          custody.custody,
          [2],
        ])
      ).publicKey;
      positionAccountsShort.push(positionAccount);
    }

    this.user = {
      wallet,
      tokenAccounts,
      lpTokenAccount: PublicKey.default,
      positionAccountsLong,
      positionAccountsShort,
    };
  };

  requestAirdrop = async (pubkey: PublicKey) => {
    if ((await this.getSolBalance(pubkey)) < 1e9 / 2) {
      return this.provider.connection.requestAirdrop(pubkey, 1e9);
    }
  };

  mintTokens = async (
    uiAmount: number,
    decimals: number,
    mint: PublicKey,
    destiantionWallet: PublicKey
  ) => {
    await spl.mintToChecked(
      this.provider.connection,
      this.admin,
      mint,
      destiantionWallet,
      this.admin,
      this.toTokenAmount(uiAmount, decimals).toNumber(),
      decimals
    );
  };

  generateCustody = async (decimals: number) => {
    let mint = Keypair.generate();
    let tokenAccount = (
      await this.findProgramAddress("custody_token_account", [
        this.pool.publicKey,
        mint.publicKey,
      ])
    ).publicKey;
    let oracleAccount = (
      await this.findProgramAddress("oracle_account", [
        this.pool.publicKey,
        mint.publicKey,
      ])
    ).publicKey;
    let custody = (
      await this.findProgramAddress("custody", [
        this.pool.publicKey,
        mint.publicKey,
      ])
    ).publicKey;
    return {
      mint,
      tokenAccount,
      oracleAccount,
      custody,
      decimals,
    };
  };

  findProgramAddress = async (label: string, extraSeeds = null) => {
    let seeds = [Buffer.from(anchor.utils.bytes.utf8.encode(label))];
    if (extraSeeds) {
      for (let extraSeed of extraSeeds) {
        if (typeof extraSeed === "string") {
          seeds.push(Buffer.from(anchor.utils.bytes.utf8.encode(extraSeed)));
        } else if (Array.isArray(extraSeed)) {
          seeds.push(Buffer.from(extraSeed));
        } else {
          seeds.push(extraSeed.toBuffer());
        }
      }
    }
    let res = await PublicKey.findProgramAddress(seeds, this.program.programId);
    return { publicKey: res[0], bump: res[1] };
  };

  confirmTx = async (txSignature: anchor.web3.TransactionSignature) => {
    const latestBlockHash = await this.provider.connection.getLatestBlockhash();

    await this.provider.connection.confirmTransaction(
      {
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: txSignature,
      },
      { commitment: "processed" }
    );
  };

  confirmAndLogTx = async (txSignature: anchor.web3.TransactionSignature) => {
    await this.confirmTx(txSignature);
    let tx = await this.provider.connection.getTransaction(txSignature, {
      commitment: "confirmed",
    });
    console.log(tx);
  };

  getBalance = async (pubkey: PublicKey) => {
    return spl
      .getAccount(this.provider.connection, pubkey)
      .then((account) => Number(account.amount))
      .catch(() => 0);
  };

  getSolBalance = async (pubkey: PublicKey) => {
    return this.provider.connection
      .getBalance(pubkey)
      .then((balance) => balance)
      .catch(() => 0);
  };

  getTime() {
    const now = new Date();
    const utcMilllisecondsSinceEpoch =
      now.getTime() + now.getTimezoneOffset() * 60 * 1000;
    return utcMilllisecondsSinceEpoch / 1000;
  }

  toTokenAmount(uiAmount: number, decimals: number) {
    return new BN(uiAmount * 10 ** decimals);
  }

  ///////
  // instructions

  init = async () => {
    try {
      await this.program.methods
        .init({})
        .accounts({
          admin: this.admin.publicKey,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([this.admin])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  addPool = async (name) => {
    try {
      await this.program.methods
        .addPool({ name })
        .accounts({
          admin: this.admin.publicKey,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          lpTokenMint: this.lpToken.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([this.admin])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }

    // set lp token account
    let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
      this.provider.connection,
      this.admin,
      this.lpToken.publicKey,
      this.user.wallet.publicKey
    );
    this.user.lpTokenAccount = tokenAccount.address;
  };

  addToken = async (custody, oracleConfig, pricingConfig) => {
    try {
      await this.program.methods
        .addToken({
          oracle: oracleConfig,
          pricing: pricingConfig,
        })
        .accounts({
          admin: this.admin.publicKey,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyTokenAccount: custody.tokenAccount,
          custodyTokenMint: custody.mint.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([this.admin])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  setTestOraclePrice = async (price: typeof BN, custody) => {
    try {
      await this.program.methods
        .setTestOraclePrice({
          price,
          expo: -3,
          conf: new BN(0),
          publishTime: new BN(this.getTime()),
        })
        .accounts({
          admin: this.admin.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          oracleAccount: custody.oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([this.admin])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  swap = async (
    amountIn: typeof BN,
    minAmountOut: typeof BN,
    user,
    fundingAccount: PublicKey,
    receivingAccount: PublicKey,
    custodyIn,
    custodyOut
  ) => {
    try {
      await this.program.methods
        .swap({
          amountIn,
          minAmountOut,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount: fundingAccount,
          receivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          receivingCustody: custodyIn.custody,
          receivingCustodyOracleAccount: custodyIn.oracleAccount,
          receivingCustodyTokenAccount: custodyIn.tokenAccount,
          dispensingCustody: custodyOut.custody,
          dispensingCustodyOracleAccount: custodyOut.oracleAccount,
          dispensingCustodyTokenAccount: custodyOut.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  addLiquidity = async (
    amount: typeof BN,
    user,
    fundingAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .addLiquidity({
          amount,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount,
          lpTokenAccount: user.lpTokenAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          lpTokenMint: this.lpToken.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(this.custodyMetas)
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  removeLiquidity = async (
    lpAmount: typeof BN,
    user,
    receivingAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .removeLiquidity({
          lpAmount,
        })
        .accounts({
          owner: user.wallet.publicKey,
          receivingAccount: receivingAccount,
          lpTokenAccount: user.lpTokenAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          lpTokenMint: this.lpToken.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(this.custodyMetas)
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  openPosition = async (
    price: typeof BN,
    collateral: typeof BN,
    size: typeof BN,
    side: PositionSide,
    user,
    fundingAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .openPosition({
          price,
          collateral,
          size,
          side: side === "long" ? { long: {} } : { short: {} },
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  closePosition = async (
    price: typeof BN,
    size: typeof BN,
    user,
    receivingAccount,
    positionAccount,
    custody
  ) => {
    try {
      await this.program.methods
        .closePosition({
          price: price,
          size: size,
        })
        .accounts({
          owner: user.wallet.publicKey,
          receivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };

  liquidate = async (
    user,
    receivingAccount: PublicKey,
    rewardReceivingAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .liquidate({})
        .accounts({
          signer: user.wallet.publicKey,
          receivingAccount,
          rewardReceivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      console.log(err);
      throw err;
    }
  };
}
