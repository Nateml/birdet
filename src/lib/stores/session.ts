import { writable, get } from 'svelte/store';
import { getNextQuestion, submitAnswer, type QuestionDto } from '$lib/api/quiz';

export type Mode = 'multiple' | 'type';

export type SessionConfig = {
    pack: string;
    length: number;
    mode: Mode;
};

export type AnswerRecord = {
    bird_id: number;
    guess: string;
    correct: boolean;
    correct_name: string;
};

export type SessionState = {
    id: string;
    config: SessionConfig;
    index: number; // 0-based question pointer
    answers: AnswerRecord[];
    current: QuestionDto | null; // current question, choices already shuffled
    finished: boolean;
};

export const session = writable<SessionState | null>(null);

// Fisher–Yates; Rust appends the correct answer last, so the UI shuffles.
function shuffle<T>(arr: T[]): T[] {
    const out = [...arr];
    for (let i = out.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [out[i], out[j]] = [out[j], out[i]];
    }
    return out;
}

export function startSession(id: string, config: SessionConfig) {
    session.set({
        id,
        config,
        index: 0,
        answers: [],
        current: null,
        finished: false
    });
}

// Fetch the next question and stash it as `current`. Returns it for convenience.
export async function loadQuestion(): Promise<QuestionDto | null> {
    const s = get(session);
    if (!s || s.finished) return null;

    const q = await getNextQuestion(s.config.pack || undefined);
    const shuffled: QuestionDto = { ...q, choices: shuffle(q.choices) };
    session.update((cur) => (cur ? { ...cur, current: shuffled } : cur));
    return shuffled;
}

// Submit the guess for the current question, record the result, advance.
export async function answerCurrent(guess: string): Promise<AnswerRecord | null> {
    const s = get(session);
    if (!s || !s.current) return null;

    const result = await submitAnswer(s.current.bird_id, s.current.recording_id, guess);
    const record: AnswerRecord = {
        bird_id: s.current.bird_id,
        guess,
        correct: result.correct,
        correct_name: result.correct_name
    };

    session.update((cur) => {
        if (!cur) return cur;
        const index = cur.index + 1;
        return {
            ...cur,
            answers: [...cur.answers, record],
            current: null,
            index,
            finished: index >= cur.config.length
        };
    });

    return record;
}

export function score(s: SessionState): number {
    return s.answers.filter((a) => a.correct).length;
}

export function endSession() {
    session.set(null);
}
