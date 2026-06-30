import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::Question`
export type QuestionDto = {
    bird_id: number;
    recording_id: number;
    choices: string[];
};

// Mirrors Rust `commands::AnswerResult`
export type AnswerResult = {
    correct: boolean;
    correct_name: string;
};

export async function getNextQuestion(pack?: string): Promise<QuestionDto> {
    return await invoke<QuestionDto>('get_next_question', { pack: pack ?? null });
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
