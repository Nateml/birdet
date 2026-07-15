import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::Question`
export type QuestionDto = {
    bird_id: number;
    recording_id: number;
    choices: string[];
    is_new: boolean;
    // Attribution for the played recording (shown after the answer is revealed).
    source: string | null;
    xc_id: string | null;
    recordist: string | null;
    license_url: string | null;
    location: string | null;
    // Other species audible in the clip (never offered as wrong options).
    background: { scientific: string; common: string | null }[];
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

// Mirrors Rust `commands::QueueCounts`. `to_go` is the headline number (min
// questions to finish if all correct); new/learning/due back the breakdown.
export type QueueCounts = {
    new: number;
    learning: number;
    due: number;
    to_go: number;
};

export async function getQueueCounts(
    pack: string | undefined,
    newRemaining: number,
    includeReviews: boolean
): Promise<QueueCounts> {
    return await invoke<QueueCounts>('get_queue_counts', {
        pack: pack ?? null,
        newRemaining,
        includeReviews
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
