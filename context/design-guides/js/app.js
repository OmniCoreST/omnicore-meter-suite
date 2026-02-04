/**
 * OMNICORE Meter Suite - Main Application
 * Handles UI interactions and backend communication
 */

(function() {
    'use strict';

    // Configuration
    const CONFIG = {
        darkModeStorageKey: 'omnicore-dark-mode',
        languageStorageKey: 'omnicore-language',
        mobileNavStorageKey: 'omnicore-mobile-nav-open',
        commLogStorageKey: 'omnicore-comm-log-open'
    };

    // State Management
    const state = {
        darkMode: false,
        currentLanguage: 'en',
        mobileNavOpen: false,
        commLogOpen: true
    };

    /**
     * Initialize the application
     */
    function init() {
        console.log('OMNICORE Meter Suite initializing...');

        // Load saved preferences
        loadPreferences();

        // Set up event listeners
        setupEventListeners();

        // Apply initial UI state
        applyDarkMode();
        applyResponsiveState();

        // Send initialization message to backend
        sendMessageToBackend('Frontend initialized successfully');

        console.log('OMNICORE Meter Suite initialized');
    }

    /**
     * Load user preferences from localStorage
     */
    function loadPreferences() {
        const savedDarkMode = localStorage.getItem(CONFIG.darkModeStorageKey);
        if (savedDarkMode !== null) {
            state.darkMode = savedDarkMode === 'true';
        }

        const savedLanguage = localStorage.getItem(CONFIG.languageStorageKey);
        if (savedLanguage) {
            state.currentLanguage = savedLanguage;
        }

        const savedCommLog = localStorage.getItem(CONFIG.commLogStorageKey);
        if (savedCommLog !== null) {
            state.commLogOpen = savedCommLog === 'true';
        }
    }

    /**
     * Set up all event listeners
     */
    function setupEventListeners() {
        // Dark mode toggle
        const darkModeToggle = document.querySelector('[class*="dark_mode"], [class*="light_mode"]')?.closest('button');
        if (darkModeToggle) {
            darkModeToggle.addEventListener('click', toggleDarkMode);
        }

        // Language switchers
        setupLanguageSwitchers();

        // Navigation menu items
        setupNavigationMenu();

        // Feature cards and interactive elements
        setupInteractiveElements();

        // Hamburger menu for mobile/responsive
        setupHamburgerMenu();

        // Communication log responsive behavior
        setupCommunicationLog();

        // Window resize handler for responsive behavior
        window.addEventListener('resize', handleWindowResize);

        // Listen for messages from C# backend
        window.addEventListener('message', handleBackendMessage);
    }

    /**
     * Toggle dark mode
     */
    function toggleDarkMode() {
        state.darkMode = !state.darkMode;
        localStorage.setItem(CONFIG.darkModeStorageKey, state.darkMode.toString());
        applyDarkMode();
        sendMessageToBackend(`Dark mode ${state.darkMode ? 'enabled' : 'disabled'}`);
    }

    /**
     * Apply dark mode to document
     */
    function applyDarkMode() {
        if (state.darkMode) {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
    }

    /**
     * Set up language switchers
     */
    function setupLanguageSwitchers() {
        const languageButtons = document.querySelectorAll('[aria-current="page"]')?.parentElement?.querySelectorAll('button');
        if (languageButtons) {
            languageButtons.forEach(button => {
                button.addEventListener('click', function() {
                    const lang = this.textContent.trim().toLowerCase();
                    switchLanguage(lang);
                });
            });
        }
    }

    /**
     * Switch application language
     */
    function switchLanguage(lang) {
        state.currentLanguage = lang;
        localStorage.setItem(CONFIG.languageStorageKey, lang);
        sendMessageToBackend(`Language switched to: ${lang}`);
        // TODO: Implement actual language switching logic
        console.log(`Language switched to: ${lang}`);
    }

    /**
     * Set up navigation menu
     */
    function setupNavigationMenu() {
        const navItems = document.querySelectorAll('nav a');
        navItems.forEach(item => {
            item.addEventListener('click', function(e) {
                e.preventDefault();
                const pageName = this.querySelector('span:last-child')?.textContent || 'Unknown';
                handleNavigation(pageName);
            });
        });
    }

    /**
     * Handle navigation
     */
    function handleNavigation(pageName) {
        console.log(`Navigating to: ${pageName}`);
        sendMessageToBackend(`Navigation: ${pageName}`);
        // TODO: Implement actual page navigation logic
    }

    /**
     * Set up interactive elements
     */
    function setupInteractiveElements() {
        // Connect Now button
        const connectButton = document.querySelector('button [class*="power_settings_new"]')?.closest('button');
        if (connectButton) {
            connectButton.addEventListener('click', handleConnectMeter);
        }

        // Previous Sessions replay buttons
        const replayButtons = document.querySelectorAll('[title="Reconnect"], [title="Retry"]');
        replayButtons.forEach(button => {
            button.addEventListener('click', function() {
                const meterInfo = this.closest('.group')?.querySelector('.font-bold')?.textContent || 'Unknown';
                handleReconnectMeter(meterInfo);
            });
        });

        // Report action buttons
        const reportButtons = document.querySelectorAll('[title="View"], [title="Download"]');
        reportButtons.forEach(button => {
            button.addEventListener('click', function() {
                const action = this.getAttribute('title');
                const fileName = this.closest('.group')?.querySelector('.font-bold')?.textContent || 'Unknown';
                handleReportAction(action, fileName);
            });
        });

        // Console log buttons
        const clearConsoleBtn = document.querySelector('[title="Clear Console"]');
        if (clearConsoleBtn) {
            clearConsoleBtn.addEventListener('click', handleClearConsole);
        }

        const exportLogBtn = document.querySelector('[title="Export Log"]');
        if (exportLogBtn) {
            exportLogBtn.addEventListener('click', handleExportLog);
        }

        // Form inputs - Connection parameters
        setupConnectionParameterListeners();
    }

    /**
     * Set up connection parameter change listeners
     */
    function setupConnectionParameterListeners() {
        const selects = document.querySelectorAll('select');
        const inputs = document.querySelectorAll('input[type="number"]');

        selects.forEach(select => {
            select.addEventListener('change', function() {
                const label = this.closest('.flex-col')?.querySelector('label')?.textContent || 'Parameter';
                sendMessageToBackend(`${label} changed to: ${this.value}`);
            });
        });

        inputs.forEach(input => {
            input.addEventListener('change', function() {
                const label = this.closest('.flex-col')?.querySelector('label')?.textContent || 'Parameter';
                sendMessageToBackend(`${label} changed to: ${this.value}`);
            });
        });
    }

    /**
     * Handle Connect Meter button click
     */
    function handleConnectMeter() {
        console.log('Initiating meter connection...');

        // Gather connection parameters
        const params = gatherConnectionParameters();

        // Send to backend
        sendMessageToBackend(JSON.stringify({
            action: 'connect',
            parameters: params
        }));

        // TODO: Show loading state, handle response
    }

    /**
     * Gather connection parameters from form
     */
    function gatherConnectionParameters() {
        const selects = document.querySelectorAll('select');
        const inputs = document.querySelectorAll('input[type="number"]');

        const params = {};

        selects.forEach(select => {
            const label = select.closest('.flex-col')?.querySelector('label')?.textContent?.trim() || '';
            if (label) {
                params[label] = select.value;
            }
        });

        inputs.forEach(input => {
            const label = input.closest('.flex-col')?.querySelector('label')?.textContent?.trim() || '';
            if (label) {
                params[label] = input.value;
            }
        });

        return params;
    }

    /**
     * Handle reconnect to meter
     */
    function handleReconnectMeter(meterInfo) {
        console.log(`Reconnecting to: ${meterInfo}`);
        sendMessageToBackend(JSON.stringify({
            action: 'reconnect',
            meter: meterInfo
        }));
    }

    /**
     * Handle report actions
     */
    function handleReportAction(action, fileName) {
        console.log(`${action} report: ${fileName}`);
        sendMessageToBackend(JSON.stringify({
            action: action.toLowerCase(),
            file: fileName
        }));
    }

    /**
     * Handle clear console
     */
    function handleClearConsole() {
        console.log('Clearing console log...');
        sendMessageToBackend(JSON.stringify({
            action: 'clearConsole'
        }));
        // TODO: Clear the console log display
    }

    /**
     * Handle export log
     */
    function handleExportLog() {
        console.log('Exporting console log...');
        sendMessageToBackend(JSON.stringify({
            action: 'exportLog'
        }));
    }

    /**
     * Set up hamburger menu for responsive navigation
     */
    function setupHamburgerMenu() {
        const hamburgerBtn = document.getElementById('hamburgerBtn');
        const mobileNavOverlay = document.getElementById('mobile-nav-overlay');
        const mobileNavSidebar = document.getElementById('mobile-nav-sidebar');

        if (!hamburgerBtn || !mobileNavOverlay) {
            return;
        }

        // Hamburger button click - open menu
        hamburgerBtn.addEventListener('click', function() {
            state.mobileNavOpen = true;
            mobileNavOverlay.classList.add('active');
            document.body.style.overflow = 'hidden';
        });

        // Overlay click - close menu
        mobileNavOverlay.addEventListener('click', function(e) {
            if (e.target === mobileNavOverlay) {
                closeMobileNav();
            }
        });

        // Mobile nav link clicks - close menu and navigate
        const mobileNavLinks = mobileNavSidebar?.querySelectorAll('a');
        mobileNavLinks?.forEach(link => {
            link.addEventListener('click', function(e) {
                e.preventDefault();
                const pageName = this.querySelector('span:last-child')?.textContent || 'Unknown';
                closeMobileNav();
                handleNavigation(pageName);
            });
        });

        // ESC key to close mobile nav
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape' && state.mobileNavOpen) {
                closeMobileNav();
            }
        });
    }

    /**
     * Close mobile navigation
     */
    function closeMobileNav() {
        const mobileNavOverlay = document.getElementById('mobile-nav-overlay');
        if (mobileNavOverlay) {
            state.mobileNavOpen = false;
            mobileNavOverlay.classList.remove('active');
            document.body.style.overflow = '';
        }
    }

    /**
     * Set up communication log responsive behavior
     */
    function setupCommunicationLog() {
        const commLogSection = document.querySelector('.communication-log-section');
        const commLogDetails = commLogSection?.querySelector('details');

        if (!commLogDetails) {
            return;
        }

        // Listen for toggle events
        commLogDetails.addEventListener('toggle', function() {
            state.commLogOpen = this.open;
            localStorage.setItem(CONFIG.commLogStorageKey, state.commLogOpen.toString());
        });
    }

    /**
     * Apply responsive state based on window size
     */
    function applyResponsiveState() {
        const width = window.innerWidth;
        const commLogSection = document.querySelector('.communication-log-section');
        const commLogDetails = commLogSection?.querySelector('details');

        if (!commLogDetails) {
            return;
        }

        // At 1366-1919px breakpoint, collapse communication log by default
        if (width >= 1366 && width < 1920) {
            // Only auto-collapse on first load if no saved preference
            const savedCommLog = localStorage.getItem(CONFIG.commLogStorageKey);
            if (savedCommLog === null) {
                commLogDetails.open = false;
                state.commLogOpen = false;
            } else {
                commLogDetails.open = state.commLogOpen;
            }
        } else {
            // At other breakpoints, use saved preference or default to open
            commLogDetails.open = state.commLogOpen;
        }
    }

    /**
     * Handle window resize
     */
    function handleWindowResize() {
        // Close mobile nav if window is resized to larger breakpoint
        if (window.innerWidth >= 1920 && state.mobileNavOpen) {
            closeMobileNav();
        }

        // Reapply responsive state
        applyResponsiveState();
    }

    /**
     * Send message to C# backend via Photino
     */
    function sendMessageToBackend(message) {
        if (window.external && window.external.sendMessage) {
            window.external.sendMessage(message);
        } else {
            console.warn('Backend communication not available:', message);
        }
    }

    /**
     * Handle messages from C# backend
     */
    function handleBackendMessage(event) {
        console.log('Message received from backend:', event.data);
        // TODO: Process backend messages and update UI accordingly
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

})();
