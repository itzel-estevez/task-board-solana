import * as anchor from "@coral-xyz/anchor";
import BN from "bn.js";
import assert from "assert";
import * as web3 from "@solana/web3.js";
import type { TaskBoard } from "../target/types/task_board";
describe("task_board", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.TaskBoard as anchor.Program<TaskBoard>;
  
  const authority = program.provider.publicKey;
  const systemProgram = web3.SystemProgram.programId;

  const [boardPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("board"), authority.toBuffer()],
    program.programId
  );

  const taskPda = (id: number) =>
    web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("task"),
        boardPda.toBuffer(),
        new BN(id).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

  const show = (label: string, sig: string) =>
    console.log(
      label + ": https://explorer.solana.com/tx/" + sig + "?cluster=devnet"
    );

  const errorCode = (e: any): string =>
    e?.error?.errorCode?.code ?? String(e?.message ?? e);

  const waitFor = async (check: () => Promise<boolean>) => {
    for (let i = 0; i < 30; i++) {
      if (await check()) return;
      await program.provider.connection.getLatestBlockhash("finalized");
    }
  };

  let taskId = 0;

  it("crea el tablero", async () => {
    const existing = await program.account.board.fetchNullable(boardPda);
    if (existing) return;
    const sig = await program.methods
      .createBoard("Mi tablero")
      .accounts({ board: boardPda, authority, systemProgram })
      .rpc();
    show("create_board", sig);
    await waitFor(
      async () =>
        (await program.account.board.fetchNullable(boardPda)) !== null
    );
    const board = await program.account.board.fetch(boardPda);
    assert.equal(board.name, "Mi tablero");
  });

  it("crea una tarea y lee sus datos", async () => {
    const board = await program.account.board.fetch(boardPda);
    taskId = board.taskCount.toNumber();
    const task = taskPda(taskId);
    const sig = await program.methods
      .createTask("Preparar entrega final")
      .accounts({ board: boardPda, task, authority, systemProgram })
      .rpc();
    show("create_task", sig);
    await waitFor(
      async () => (await program.account.task.fetchNullable(task)) !== null
    );
    const data = await program.account.task.fetch(task);
    assert.equal(data.title, "Preparar entrega final");
    assert.equal(data.completed, false);
  });

  it("rechaza un titulo vacio", async () => {
    try {
      await program.methods
        .createTask("   ")
        .accounts({
          board: boardPda,
          task: taskPda(taskId + 1),
          authority,
          systemProgram,
        })
        .rpc();
      assert.fail("debio fallar");
    } catch (e) {
      assert.equal(errorCode(e), "EmptyText");
    }
  });

  it("modifica el titulo de la tarea", async () => {
    const task = taskPda(taskId);
    const sig = await program.methods
      .updateTask("Entrega final en Devnet")
      .accounts({ board: boardPda, task, authority })
      .rpc();
    show("update_task", sig);
    await waitFor(
      async () =>
        (await program.account.task.fetch(task)).title ===
        "Entrega final en Devnet"
    );
    const data = await program.account.task.fetch(task);
    assert.equal(data.title, "Entrega final en Devnet");
  });

  it("impide que otra wallet modifique la tarea", async () => {
    const attacker = web3.Keypair.generate();
    const fund = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: authority,
        toPubkey: attacker.publicKey,
        lamports: 20000000,
      })
    );
    await program.provider.sendAndConfirm(fund);
    try {
      await program.methods
        .updateTask("Hackeada")
        .accounts({
          board: boardPda,
          task: taskPda(taskId),
          authority: attacker.publicKey,
        })
        .signers([attacker])
        .rpc();
      assert.fail("debio fallar");
    } catch (e) {
      assert.notEqual(errorCode(e), "debio fallar");
      console.log("rechazo de permisos: " + errorCode(e));
    }
  });

  it("rechaza cerrar una tarea no completada", async () => {
    try {
      await program.methods
        .closeTask()
        .accounts({ board: boardPda, task: taskPda(taskId), authority })
        .rpc();
      assert.fail("debio fallar");
    } catch (e) {
      assert.equal(errorCode(e), "TaskNotCompleted");
    }
  });

  it("completa y cierra la tarea", async () => {
    const task = taskPda(taskId);
    const completeSig = await program.methods
      .setCompleted(true)
      .accounts({ board: boardPda, task, authority })
      .rpc();
    show("set_completed", completeSig);
    await waitFor(
      async () => (await program.account.task.fetch(task)).completed === true
    );
    const closeSig = await program.methods
      .closeTask()
      .accounts({ board: boardPda, task, authority })
      .rpc();
    show("close_task", closeSig);
    await waitFor(
      async () => (await program.account.task.fetchNullable(task)) === null
    );
    const closed = await program.account.task.fetchNullable(task);
    assert.equal(closed, null);
  });
});
