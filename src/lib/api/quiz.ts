import { invoke } from '@tauri-apps/api/core';

export type QuestionDto = {
    recording_id: number;
    choices: string[];
    correct_index: number;
};

export async function getNextQuestion(session_id: string): Promise<QuestionDto> {
    return await invoke<QuestionDto>('get_next_question', { session_id });
}
