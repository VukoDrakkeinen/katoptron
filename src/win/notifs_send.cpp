// notifs_send.cpp : Defines the entry point for the application.
//

#include "stdafx.h"
#include "notifs_send.h"
#include <cstdio>

#define MAX_LOADSTRING 100

// Global Variables:
HINSTANCE hInst;                                // current instance
WCHAR window_title[MAX_LOADSTRING];                  // The title bar text
WCHAR window_class[MAX_LOADSTRING];            // the main window class name

// Forward declarations of functions included in this code module:
ATOM                MyRegisterClass(HINSTANCE hInstance);
BOOL                InitInstance(HINSTANCE, int);
LRESULT CALLBACK    WndProc(HWND, UINT, WPARAM, LPARAM);

int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
                     _In_opt_ HINSTANCE hPrevInstance,
                     _In_ LPWSTR    lpCmdLine,
                     _In_ int       nCmdShow)
{
    UNREFERENCED_PARAMETER(hPrevInstance);
    UNREFERENCED_PARAMETER(lpCmdLine);

    // TODO: Place code here.

    // Initialize global strings
    LoadStringW(hInstance, IDS_APP_TITLE, window_title, MAX_LOADSTRING);
    LoadStringW(hInstance, IDC_NOTIFSSEND, window_class, MAX_LOADSTRING);
    MyRegisterClass(hInstance);

    // Perform application initialization:
    if (!InitInstance (hInstance, nCmdShow))
    {
        return FALSE;
    }

    HACCEL hAccelTable = LoadAccelerators(hInstance, MAKEINTRESOURCE(IDC_NOTIFSSEND));

    MSG msg;
    // Main message loop:
    while (GetMessage(&msg, nullptr, 0, 0))
    {
        if (!TranslateAccelerator(msg.hwnd, hAccelTable, &msg))
        {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
    }

    return (int) msg.wParam;
}



//
//  FUNCTION: MyRegisterClass()
//
//  PURPOSE: Registers the window class.
//
ATOM MyRegisterClass(HINSTANCE hInstance)
{
    WNDCLASSEXW wcex;

    wcex.cbSize = sizeof(WNDCLASSEX);

    wcex.style          = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc    = WndProc;
    wcex.cbClsExtra     = 0;
    wcex.cbWndExtra     = 0;
    wcex.hInstance      = hInstance;
    wcex.hIcon          = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_NOTIFSSEND));
    wcex.hCursor        = LoadCursor(nullptr, IDC_ARROW);
    wcex.hbrBackground  = (HBRUSH)(COLOR_WINDOW+1);
    wcex.lpszMenuName   = MAKEINTRESOURCEW(IDC_NOTIFSSEND);
    wcex.lpszClassName  = window_class;
    wcex.hIconSm        = LoadIcon(wcex.hInstance, MAKEINTRESOURCE(IDI_SMALL));

    return RegisterClassExW(&wcex);
}

//
//   FUNCTION: InitInstance(HINSTANCE, int)
//
//   PURPOSE: Saves instance handle and creates main window
//
//   COMMENTS:
//
//        In this function, we save the instance handle in a global variable and
//        create and display the main program window.
//

#include <io.h>
#include <Fcntl.h>

UINT notify_id = -1;

BOOL InitInstance(HINSTANCE hInstance, int nCmdShow)
{
   hInst = hInstance; // Store instance handle in our global variable

   HWND hWnd = CreateWindowW(window_class, window_title, WS_OVERLAPPEDWINDOW,
      CW_USEDEFAULT, 0, CW_USEDEFAULT, 0, nullptr, nullptr, hInstance, nullptr);

   if (!hWnd)
   {
      return FALSE;
   }

   AllocConsole();
   HANDLE ConsoleOutput = GetStdHandle(STD_OUTPUT_HANDLE);
   int SystemOutput = _open_osfhandle(intptr_t(ConsoleOutput), _O_TEXT);
   FILE *COutputHandle = _fdopen(SystemOutput, "w");

   // Get STDERR handle
   HANDLE ConsoleError = GetStdHandle(STD_ERROR_HANDLE);
   int SystemError = _open_osfhandle(intptr_t(ConsoleError), _O_TEXT);
   FILE *CErrorHandle = _fdopen(SystemError, "w");

   // Get STDIN handle
   HANDLE ConsoleInput = GetStdHandle(STD_INPUT_HANDLE);
   int SystemInput = _open_osfhandle(intptr_t(ConsoleInput), _O_TEXT);
   FILE *CInputHandle = _fdopen(SystemInput, "r");

   freopen_s(&CInputHandle, "CONIN$", "r", stdin);
   freopen_s(&COutputHandle, "CONOUT$", "w", stdout);
   freopen_s(&CErrorHandle, "CONOUT$", "w", stderr);


   notify_id = RegisterWindowMessageA("SHELLHOOK");
   RegisterShellHookWindow(hWnd);

//   ShowWindow(hWnd, nCmdShow);
   UpdateWindow(hWnd);

   return TRUE;
}

//
//  FUNCTION: WndProc(HWND, UINT, WPARAM, LPARAM)
//
//  PURPOSE:  Processes messages for the main window.
//
//  WM_COMMAND  - process the application menu
//  WM_PAINT    - Paint the main window
//  WM_DESTROY  - post a quit message and return
//
//
LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam)
{
//	printf("msg: %ud\n", message);
    switch (message)
    {
    case WM_COMMAND:
        {
            int wmId = LOWORD(wParam);
            // Parse the menu selections:
            switch (wmId)
            {
            case IDM_EXIT:
                DestroyWindow(hWnd);
                break;
            default:
                return DefWindowProc(hWnd, message, wParam, lParam);
            }
        }
        break;
    case WM_DESTROY:
        PostQuitMessage(0);
        break;
	break;
    default:
		if  (message == notify_id) {
			if (wParam == HSHELL_WINDOWCREATED) {
				HWND window_handle = (HWND)lParam;

				//		int title_len = GetWindowTextLengthA(window_handle) + 1;
				char title[2048];
				char clazz[2048];
				GetWindowTextA(window_handle, title, 2048);
				GetClassNameA(window_handle, clazz, 2048);
				printf("[new] %s: %s\n", title, clazz);
			}

			if (wParam == HSHELL_FLASH) {
				HWND window_handle = (HWND)lParam;

				char title[2048];
				char clazz[2048];
				GetWindowTextA(window_handle, title, 2048);
				GetClassNameA(window_handle, clazz, 2048);
				printf("[flashing] %s: %s\n", title, clazz);
			}
		}
		return DefWindowProc(hWnd, message, wParam, lParam);
    }
    return 0; //WindowsForms10.Window.8.app.0.218f99c
}
