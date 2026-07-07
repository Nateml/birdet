import { writable, get } from 'svelte/store';
import { getNextQuestion, submitAnswer, type QuestionDto } from '$lib/api/quiz';

export type Mode = 'multiple' | 'type';

// How the SRS queue is scoped for a session:
//   mixed  — due reviews + new up to the cap (the daily driver)
//   review — clear due cards only, introduce no new
//   new    — introduce new birds only, skip the review queue
//   cram   — practice anyway: ignore the schedule, fixed `length` questions
export type StudyMode = 'mixed' | 'review' | 'new' | 'cram';

export type SessionConfig = {
    pack: string; // '' = whole library
    length: number; // Anki new-card cap: max brand-new birds introduced this session
    mode: Mode;
    study: StudyMode;
};

export type AnswerRecord = {
    bird_id: number;
    recording_id: number; // kept so results can replay the call
    guess: string;
    correct: boolean;
    correct_name: string;
    skipped: boolean;
};

export type SessionState = {
    id: string;
    config: SessionConfig;
    index: number; // 0-based question pointer (counts every card shown, incl. reviews)
    newServed: number; // brand-new birds introduced so far (against config.length cap)
    answers: AnswerRecord[];
    current: QuestionDto | null; // current question, choices already shuffled
    finished: boolean; // set once the queue drains (getNextQuestion → null)
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
        newServed: 0,
        answers: [],
        current: null,
        finished: false
    });
}

// Fetch the next card from the SRS queue and stash it as `current`. Returns null
// when the queue is drained (new budget spent + nothing due) — session is over.
export async function loadQuestion(): Promise<QuestionDto | null> {
    const s = get(session);
    if (!s || s.finished) return null;

    // Cram is a fixed-length practice run (ignores the schedule); it ends after
    // `length` questions rather than draining a queue.
    if (s.config.study === 'cram') {
        if (s.answers.length >= s.config.length) {
            session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
            return null;
        }
        const cq = await getNextQuestion(s.config.pack || undefined, 0, false, true);
        if (!cq) {
            session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
            return null;
        }
        const shuffledCram: QuestionDto = { ...cq, choices: shuffle(cq.choices) };
        session.update((cur) => (cur ? { ...cur, current: shuffledCram } : cur));
        return shuffledCram;
    }

    // 'review' spends no new-card budget; 'new' skips the review queue.
    const includeReviews = s.config.study !== 'new';
    const newRemaining =
        s.config.study === 'review' ? 0 : Math.max(0, s.config.length - s.newServed);
    const q = await getNextQuestion(s.config.pack || undefined, newRemaining, includeReviews);
    if (!q) {
        session.update((cur) => (cur ? { ...cur, current: null, finished: true } : cur));
        return null;
    }
    const shuffled: QuestionDto = { ...q, choices: shuffle(q.choices) };
    session.update((cur) => (cur ? { ...cur, current: shuffled } : cur));
    return shuffled;
}

// Submit the guess for the current question, record the result, advance.
// `skipped` marks a deliberate "I don't know" (still counts as not-correct).
export async function answerCurrent(guess: string, skipped = false): Promise<AnswerRecord | null> {
    const s = get(session);
    if (!s || !s.current) return null;

    const result = await submitAnswer(s.current.bird_id, s.current.recording_id, guess);
    const record: AnswerRecord = {
        bird_id: s.current.bird_id,
        recording_id: s.current.recording_id,
        guess,
        correct: result.correct,
        correct_name: result.correct_name,
        skipped
    };

    session.update((cur) => {
        if (!cur) return cur;
        // A new bird just answered spends one of the new-card budget. `finished`
        // is decided by loadQuestion (queue drain), not a fixed question count.
        const newServed = cur.newServed + (cur.current?.is_new ? 1 : 0);
        return {
            ...cur,
            answers: [...cur.answers, record],
            current: null,
            index: cur.index + 1,
            newServed
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
