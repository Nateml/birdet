import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::Question`
export type QuestionDto = {
    bird_id: number;
    recording_id: number;
    choices: string[];
    is_new: boolean;
};

// Mirrors Rust `commands::AnswerResult`
export type AnswerResult = {
    correct: boolean;
    correct_name: string;
};

// Returns null when the session queue is drained (no card due / new budget spent).
// `includeReviews=false` studies only new cards (a "new-only" session).
export async function getNextQuestion(
    pack: string | undefined,
    newRemaining: number,
    includeReviews: boolean,
    cram = false
): Promise<QuestionDto | null> {
    return await invoke<QuestionDto | null>('get_next_question', {
        pack: pack ?? null,
        newRemaining,
        includeReviews,
        cram
    });
}

export async function submitAnswer(
    bird_id: number,
    recording_id: number,
    guess: string
): Promise<AnswerResult> {
    return await invoke<AnswerResult>('submit_answer', {
        payload: { bird_id, recording_id, guess }
    });
}
