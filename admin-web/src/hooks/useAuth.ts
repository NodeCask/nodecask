import {useReducer, useEffect, useCallback} from "react";
import {useNavigate} from "react-router";
import {hasToken, login as apiLogin, logout as apiLogout, me as apiMe, SysError, type UserInfo} from "../api";

// --- Types ---

interface AuthState {
    isAuthenticated: boolean;
    isLoading: boolean;
    userInfo: UserInfo | null;
    error: string | null;
}

type AuthAction =
    | { type: 'START_LOADING' }
    | { type: 'LOGIN_SUCCESS'; payload: UserInfo }
    | { type: 'AUTH_FAILURE'; payload: string } // Used for login errors
    | { type: 'RESET_AUTH' } // Used for logout or init failure
    | { type: 'UPDATE_USER'; payload: UserInfo }
    | { type: 'CLEAR_ERROR' };

// --- Reducer ---

const initialState: AuthState = {
    isAuthenticated: false,
    isLoading: true, // Start in loading state for initial check
    userInfo: null,
    error: null,
};

function authReducer(state: AuthState, action: AuthAction): AuthState {
    switch (action.type) {
        case 'START_LOADING':
            return { ...state, isLoading: true, error: null };
        case 'LOGIN_SUCCESS':
            return {
                ...state,
                isLoading: false,
                isAuthenticated: true,
                userInfo: action.payload,
                error: null,
            };
        case 'AUTH_FAILURE':
            return {
                ...state,
                isLoading: false,
                isAuthenticated: false,
                userInfo: null,
                error: action.payload,
            };
        case 'RESET_AUTH':
            return {
                ...state,
                isLoading: false,
                isAuthenticated: false,
                userInfo: null,
                error: null,
            };
        case 'UPDATE_USER':
            return {
                ...state,
                userInfo: action.payload,
            };
        case 'CLEAR_ERROR':
            return { ...state, error: null };
        default:
            return state;
    }
}

// --- Hook ---

interface UseAuthReturn extends AuthState {
    login: (username: string, password: string) => Promise<boolean>;
    logout: () => Promise<void>;
    refreshUserInfo: () => Promise<void>;
    clearError: () => void;
}

export function useAuth(): UseAuthReturn {
    const navigate = useNavigate();
    const [state, dispatch] = useReducer(authReducer, initialState);

    // Initial Authentication Check
    useEffect(() => {
        const initAuth = async () => {
            if (!hasToken()) {
                dispatch({ type: 'RESET_AUTH' });
                return;
            }

            try {
                const user = await apiMe();
                dispatch({ type: 'LOGIN_SUCCESS', payload: user });
            } catch (err) {
                console.error("Auth initialization failed:", err);
                if (err instanceof SysError && err.code === 99) {
                    await apiLogout(); // Ensure local token is cleared
                    navigate("/login");
                }
                // If it's a network error, we might want to stay in a "loading" or "error" state,
                // but for now, we reset to safe default.
                dispatch({ type: 'RESET_AUTH' });
            }
        };

        initAuth();
    }, [navigate]);

    const login = useCallback(async (username: string, password: string): Promise<boolean> => {
        dispatch({ type: 'START_LOADING' });
        try {
            const resp = await apiLogin(username, password);
            if (resp.totp) {
                dispatch({ type: 'AUTH_FAILURE', payload: "Two-factor authentication required" });
                return false;
            }
            const user = await apiMe();
            dispatch({ type: 'LOGIN_SUCCESS', payload: user });
            return true;
        } catch (err) {
            let message = "Login failed, please try again.";
            if (err instanceof SysError) {
                message = err.message || "Login failed, check username and password.";
            }
            dispatch({ type: 'AUTH_FAILURE', payload: message });
            return false;
        }
    }, []);

    const logout = useCallback(async (): Promise<void> => {
        dispatch({ type: 'START_LOADING' });
        try {
            await apiLogout();
        } catch (err) {
            console.error("Logout API failed:", err);
        } finally {
            dispatch({ type: 'RESET_AUTH' });
            navigate("/login");
        }
    }, [navigate]);

    const refreshUserInfo = useCallback(async (): Promise<void> => {
        if (!state.isAuthenticated) return;

        try {
            const user = await apiMe();
            dispatch({ type: 'UPDATE_USER', payload: user });
        } catch (err) {
            console.error("Refresh user info failed:", err);
             if (err instanceof SysError && err.code === 99) {
                 // If token became invalid during refresh, force logout
                 await logout();
             }
        }
    }, [state.isAuthenticated, logout]);

    const clearError = useCallback(() => {
        dispatch({ type: 'CLEAR_ERROR' });
    }, []);

    return {
        ...state,
        login,
        logout,
        refreshUserInfo,
        clearError,
    };
}