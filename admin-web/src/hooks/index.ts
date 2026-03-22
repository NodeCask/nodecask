// hooks 索引文件

// 自定义 React hooks
export * from "./useAuth";

// 可以在这里定义通用的自定义 hooks
// 例如：useDebounce, useThrottle, useIntersectionObserver 等


// Spinner Reducer
type SpinnerState = {
    loading: boolean;
    saving: boolean;
};

type SpinnerAction =
    | { type: "START_LOADING" }
    | { type: "STOP_LOADING" }
    | { type: "START_SAVING" }
    | { type: "STOP_SAVING" };

export const spinnerReducer = (state: SpinnerState, action: SpinnerAction): SpinnerState => {
    switch (action.type) {
        case "START_LOADING":
            return { ...state, loading: true };
        case "STOP_LOADING":
            return { ...state, loading: false };
        case "START_SAVING":
            return { ...state, saving: true };
        case "STOP_SAVING":
            return { ...state, saving: false };
        default:
            return state;
    }
};

export const initialSpinnerState: SpinnerState = {
    loading: true,
    saving: false,
};
