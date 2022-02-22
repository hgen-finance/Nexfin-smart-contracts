import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { StableCoin } from '../target/types/stable_coin';
import { TOKEN_PROGRAM_ID, Token, ASSOCIATED_TOKEN_PROGRAM_ID, } from '@solana/spl-token';
import { assert } from "chai";
import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { loadWalletKey } from "./helpers/accounts";

describe('nexfin', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.StableCoin as Program<StableCoin>;

  let escrow_account_pda = null;
  let escrow_account_bump = null;
  let stable_account_pda = null;
  let stable_token = null;
  let token_authority = null;
  let token_authority_bump = null;
  let stable_account_bump = null;
  let escrow_info_pda = null;
  let escrow_info_bump = null;
  let user_escrow_info_pda = null;
  let user_escrow_info_bump = null;
  let user_stable_account = null;
  let user_escrow_pda = null;
  let user_escrow_bump = null;

  
  const userAccount = loadWalletKey("./wallet/user.json");
  const adminAccount = loadWalletKey("./wallet/admin.json");

  let pythAccount = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix")

  it('Is initialized!', async () => {
    // Add your test here.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(adminAccount.publicKey, 2000000000),
      "confirmed"
    ); 

    await provider.send(
      (() => {
        const tx = new Transaction();
        tx.add(
          SystemProgram.transfer({
            fromPubkey: adminAccount.publicKey,
            toPubkey: userAccount.publicKey,
            lamports: 1300000000,
          }),
        );
        return tx;
      })(),
      [adminAccount]
    );

    [token_authority, token_authority_bump] = await PublicKey.findProgramAddress([
      Buffer.from("mint-authority"),
    ], program.programId);

    console.log("token authoirty", token_authority.toString());

     stable_token = await Token.createMint(
       provider.connection,
       adminAccount,
       token_authority,
       null,
       9, // Decimal is 6
       TOKEN_PROGRAM_ID,
     );

    // console.log("stable token", stable_token.publicKey.toString());

    [escrow_account_pda, escrow_account_bump] = await PublicKey.findProgramAddress([
      Buffer.from("escrow"),
    ], program.programId);

    console.log("escrow: ", escrow_account_pda.toString());

    const escrow_amount = "500000000";

    [stable_account_pda, stable_account_bump] = await PublicKey.findProgramAddress([
      Buffer.from("stable-token-account"),
    ], program.programId);

    console.log("stable account: ", stable_account_pda.toString());

    [escrow_info_pda, escrow_info_bump] = await PublicKey.findProgramAddress([
      Buffer.from("escrow-info"),
    ], program.programId);

    console.log("escrow info: ", escrow_info_pda.toString());

    // Add your test here.
    await program.rpc.escrowSol(
      escrow_account_bump,
      stable_account_bump,
      escrow_info_bump,
      token_authority_bump,
      new anchor.BN(escrow_amount),
      {
        accounts: {
          adminAccount: adminAccount.publicKey,
          stableToken: stable_token.publicKey,
          tokenAuthority: token_authority,
          escrowAccount: escrow_account_pda,
          stableAccount: stable_account_pda,
          escrowInfoAccount: escrow_info_pda,
          pythAccount: pythAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [adminAccount]
      }
    );


  });

  it('Mint Burn tokens', async () => {
    sleep(2000);
    stable_token = new PublicKey("Cfh5iBgxUZbxgqAEeojxdtt28CwqqaMSb2gzPq8J2LUJ");
    await program.rpc.mintBurnStableToken(
      token_authority_bump,
      {
        accounts: {
          adminAccount: adminAccount.publicKey,
          stableToken: stable_token,
          tokenAuthority: token_authority,
          stableAccount: stable_account_pda,
          escrowInfoAccount: escrow_info_pda,
          pythAccount: pythAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [adminAccount]
      }
    );    
  });

  // it('Init User', async () => {
  //   [user_escrow_info_pda, user_escrow_info_bump] = await PublicKey.findProgramAddress([
  //     Buffer.from("user-escrow-info"),
  //     userAccount.publicKey.toBuffer()
  //   ], program.programId);

  //   console.log("user escrow info: " + user_escrow_info_pda.toString());

  //   await program.rpc.initUser(
  //     user_escrow_info_bump,
  //     {
  //       accounts: {
  //         userAccount: userAccount.publicKey,
  //         userEscrowInfoAccount: user_escrow_info_pda,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //         rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //       },
  //       signers: [userAccount]
  //     }
  //   );
  // });

  // it('User escrow SOL', async () => {
  //   stable_token = await Token.createMint(
  //     provider.connection,
  //     adminAccount,
  //     token_authority,
  //     null,
  //     9, // Decimal is 6
  //     TOKEN_PROGRAM_ID,
  //   );

  //   console.log('stable token: ', stable_token.publicKey.toString());

  //   user_stable_account = await stable_token.createAccount(userAccount.publicKey);
  //   console.log('user stable account: ', user_stable_account.toString());

  //   [user_escrow_info_pda, user_escrow_info_bump] = await PublicKey.findProgramAddress([
  //     Buffer.from("user-escrow-info"),
  //     userAccount.publicKey.toBuffer()
  //   ], program.programId);

  //   [user_escrow_pda, user_escrow_bump] = await PublicKey.findProgramAddress([
  //     Buffer.from("user-escrow"),
  //   ], program.programId);

  //   const user_escrow_amount = "1000000000";

  //   await program.rpc.userEscrowSol(
  //     token_authority_bump,
  //     new anchor.BN(user_escrow_amount),
  //     {
  //       accounts: {
  //         userAccount: userAccount.publicKey,
  //         stableToken: stable_token.publicKey,
  //         tokenAuthority: token_authority,
  //         escrowAccount: user_escrow_pda,
  //         userStableAccount: user_stable_account,
  //         userEscrowInfoAccount: user_escrow_info_pda,
  //         pythAccount: pythAccount,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [userAccount]
  //     }
  //   );

  //   let _escrow = await provider.connection.getAccountInfo(user_escrow_pda);
  //   console.log(_escrow);

  //   let _escrow_info = await program.account.userEscrowInfoAccount.fetch(user_escrow_info_pda);
  //   console.log(_escrow_info.escrowSolAmount.toNumber());
  // });

  // it('User withdraw SOL', async () => {
  //   await program.rpc.userWithdrawSol(
  //     {
  //       accounts: {
  //         userAccount: userAccount.publicKey,
  //         stableToken: stable_token.publicKey,
  //         tokenAuthority: token_authority,
  //         escrowAccount: user_escrow_pda,
  //         userStableAccount: user_stable_account,
  //         userEscrowInfoAccount: user_escrow_info_pda,
  //         pythAccount: pythAccount,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [userAccount]
  //     }
  //   );

  //   let _escrow = await provider.connection.getAccountInfo(user_escrow_pda);
  //   console.log(_escrow);

  //   let _temp = await stable_token.getAccountInfo(user_stable_account);
  //   console.log(_temp.amount.toNumber());

  // });

  // it('Close user', async () => {
  //  [user_escrow_info_pda, user_escrow_info_bump] = await PublicKey.findProgramAddress([
  //    Buffer.from("user-escrow-info"),
  //    userAccount.publicKey.toBuffer()
  //  ], program.programId);

  //  await program.rpc.closeUserEscrow(
  //    {
  //      accounts: {
  //        userAccount: userAccount.publicKey,
  //        userEscrowInfoAccount: user_escrow_info_pda,
  //      },
  //      signers: [userAccount]
  //    }
  //  );
  //});
});

function sleep(milliseconds) {
  const date = Date.now();
  let currentDate = null;
  do {
    currentDate = Date.now();
  } while (currentDate - date < milliseconds);
}
