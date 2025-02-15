// No imports needed: web3, anchor, pg, and BN are globally available

describe("mryt_protocol tests", () => {
  // Generate keypairs for accounts
  let configKeypair = new web3.Keypair();
  let mrytMintKeypair = new web3.Keypair();
  let lpMintKeypair = new web3.Keypair();
  
  let userLpTokenAccount;
  let vaultLpTokenAccount;
  let userMrytTokenAccount;

  let stakedPositionPda;
  let depositAmount = new BN(1000);

  // SPL Token Program ID 
  const TOKEN_PROGRAM_ID = new web3.PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

  before(async () => {
    // Derive PDA for staked position using ["staked", wallet.publicKey]
    [stakedPositionPda] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("staked"), pg.wallet.publicKey.toBuffer()],
      pg.program.programId
    );

    // Create token accounts
    userLpTokenAccount = new web3.Keypair();
    vaultLpTokenAccount = new web3.Keypair();
    userMrytTokenAccount = new web3.Keypair();
  });

  it("Initializes the protocol", async () => {
    const txHash = await pg.program.methods
      .initialize()
      .accounts({
        config: configKeypair.publicKey,
        authority: pg.wallet.publicKey,
        mrytMint: mrytMintKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID, 
      })
      .signers([configKeypair, mrytMintKeypair])
      .rpc();
    console.log(`Initialized (tx: ${txHash})`);

    const configAccount = await pg.program.account.config.fetch(
      configKeypair.publicKey
    );
    assert(configAccount.totalStaked.eq(new BN(0)));
    assert(configAccount.totalYield.eq(new BN(0)));
    assert(configAccount.totalMrytSupply.eq(new BN(0)));
  });

  it("Deposits LP tokens and mints MRYT", async () => {
    const txHash = await pg.program.methods
      .deposit(depositAmount)
      .accounts({
        config: configKeypair.publicKey,
        authority: pg.wallet.publicKey,
        userTokenAccount: userLpTokenAccount.publicKey,
        vaultTokenAccount: vaultLpTokenAccount.publicKey,
        mrytMint: mrytMintKeypair.publicKey,
        userMryt: userMrytTokenAccount.publicKey,
        stakedPosition: stakedPositionPda,
        tokenProgram: TOKEN_PROGRAM_ID, 
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([userLpTokenAccount])
      .rpc();
    console.log(`Deposit tx: ${txHash}`);

    const configAccount = await pg.program.account.config.fetch(
      configKeypair.publicKey
    );
    assert(configAccount.totalStaked.eq(depositAmount));
  });

  it("Accrues yield", async () => {
    const txHash = await pg.program.methods
      .accrueYield()
      .accounts({
        config: configKeypair.publicKey,
      })
      .rpc();
    console.log(`Accrue yield tx: ${txHash}`);

    const configAccount = await pg.program.account.config.fetch(
      configKeypair.publicKey
    );
    assert(configAccount.totalYield.gt(new BN(0))); // Ensure yield increased
  });

  it("Auto-compounds yield", async () => {
    const txHash = await pg.program.methods
      .autoCompoundYield()
      .accounts({
        config: configKeypair.publicKey,
      })
      .rpc();
    console.log(`Auto-compound tx: ${txHash}`);

    const configAccount = await pg.program.account.config.fetch(
      configKeypair.publicKey
    );
    assert(configAccount.totalStaked.gt(depositAmount));
  });

  it("Calculates APY", async () => {
    const txHash = await pg.program.methods
      .calculateApy()
      .accounts({
        config: configKeypair.publicKey,
      })
      .rpc();
    console.log(`Calculate APY tx: ${txHash}`);
  });

  it("Prevents early withdrawal", async () => {
    try {
      await pg.program.methods
        .withdraw(new BN(10))
        .accounts({
          config: configKeypair.publicKey,
          authority: pg.wallet.publicKey,
          userMryt: userMrytTokenAccount.publicKey,
          mrytMint: mrytMintKeypair.publicKey,
          vaultTokenAccount: vaultLpTokenAccount.publicKey,
          userTokenAccount: userLpTokenAccount.publicKey,
          stakedPosition: stakedPositionPda,
          tokenProgram: TOKEN_PROGRAM_ID, 
        })
        .rpc();
      assert.fail("Withdrawal should have failed due to early withdrawal");
    } catch (err) {
      console.log("Early withdrawal prevented as expected:", err.toString());
      assert(err.toString().includes("EarlyWithdrawal"));
    }
  });
});
