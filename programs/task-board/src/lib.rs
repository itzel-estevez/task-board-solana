use anchor_lang::prelude::*;

declare_id!("HNqAMQ5kmYwFeLqK6VPkYNKVJyYN7Eb8wNDGg4gucDas");

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_TITLE_LEN: usize = 64;

#[program]
pub mod task_board {
    use super::*;

    pub fn create_board(ctx: Context<CreateBoard>, name: String) -> Result<()> {
        require!(!name.trim().is_empty(), TaskBoardError::EmptyText);
        require!(name.len() <= MAX_NAME_LEN, TaskBoardError::TextTooLong);

        let board = &mut ctx.accounts.board;
        board.authority = ctx.accounts.authority.key();
        board.name = name;
        board.task_count = 0;
        board.bump = ctx.bumps.board;
        Ok(())
    }

    pub fn create_task(ctx: Context<CreateTask>, title: String) -> Result<()> {
        require!(!title.trim().is_empty(), TaskBoardError::EmptyText);
        require!(title.len() <= MAX_TITLE_LEN, TaskBoardError::TextTooLong);

        let board = &mut ctx.accounts.board;
        let task = &mut ctx.accounts.task;
        task.board = board.key();
        task.id = board.task_count;
        task.title = title;
        task.completed = false;
        task.created_at = Clock::get()?.unix_timestamp;
        task.bump = ctx.bumps.task;

        board.task_count = board
            .task_count
            .checked_add(1)
            .ok_or(TaskBoardError::Overflow)?;
        Ok(())
    }

    pub fn update_task(ctx: Context<ModifyTask>, new_title: String) -> Result<()> {
        require!(!new_title.trim().is_empty(), TaskBoardError::EmptyText);
        require!(new_title.len() <= MAX_TITLE_LEN, TaskBoardError::TextTooLong);

        ctx.accounts.task.title = new_title;
        Ok(())
    }

    pub fn set_completed(ctx: Context<ModifyTask>, completed: bool) -> Result<()> {
        ctx.accounts.task.completed = completed;
        Ok(())
    }

    pub fn close_task(_ctx: Context<CloseTask>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateBoard<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Board::INIT_SPACE,
        seeds = [b"board", authority.key().as_ref()],
        bump
    )]
    pub board: Account<'info, Board>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTask<'info> {
    #[account(
        mut,
        has_one = authority @ TaskBoardError::Unauthorized,
        seeds = [b"board", authority.key().as_ref()],
        bump = board.bump
    )]
    pub board: Account<'info, Board>,
    #[account(
        init,
        payer = authority,
        space = 8 + Task::INIT_SPACE,
        seeds = [b"task", board.key().as_ref(), board.task_count.to_le_bytes().as_ref()],
        bump
    )]
    pub task: Account<'info, Task>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ModifyTask<'info> {
    #[account(
        has_one = authority @ TaskBoardError::Unauthorized,
        seeds = [b"board", authority.key().as_ref()],
        bump = board.bump
    )]
    pub board: Account<'info, Board>,
    #[account(
        mut,
        has_one = board @ TaskBoardError::WrongBoard,
        seeds = [b"task", board.key().as_ref(), task.id.to_le_bytes().as_ref()],
        bump = task.bump
    )]
    pub task: Account<'info, Task>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseTask<'info> {
    #[account(
        has_one = authority @ TaskBoardError::Unauthorized,
        seeds = [b"board", authority.key().as_ref()],
        bump = board.bump
    )]
    pub board: Account<'info, Board>,
    #[account(
        mut,
        close = authority,
        has_one = board @ TaskBoardError::WrongBoard,
        constraint = task.completed @ TaskBoardError::TaskNotCompleted,
        seeds = [b"task", board.key().as_ref(), task.id.to_le_bytes().as_ref()],
        bump = task.bump
    )]
    pub task: Account<'info, Task>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Board {
    pub authority: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub task_count: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Task {
    pub board: Pubkey,
    pub id: u64,
    #[max_len(64)]
    pub title: String,
    pub completed: bool,
    pub created_at: i64,
    pub bump: u8,
}

#[error_code]
pub enum TaskBoardError {
    #[msg("El texto no puede estar vacio")]
    EmptyText,
    #[msg("El texto excede la longitud maxima")]
    TextTooLong,
    #[msg("Solo la autoridad del tablero puede realizar esta accion")]
    Unauthorized,
    #[msg("La tarea no pertenece a este tablero")]
    WrongBoard,
    #[msg("Solo se pueden cerrar tareas completadas")]
    TaskNotCompleted,
    #[msg("Desbordamiento aritmetico")]
    Overflow,
}