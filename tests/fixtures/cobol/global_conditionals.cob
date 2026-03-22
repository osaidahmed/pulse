       IDENTIFICATION DIVISION.
       PROGRAM-ID. GLOBAL-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-MODE PIC 9.
       01 WS-DEBUG PIC 9.
       01 WS-VALUE PIC 9(3).
       PROCEDURE DIVISION.
           IF WS-MODE = 1
               DISPLAY "Mode 1"
           END-IF.
           IF WS-DEBUG = 1
               DISPLAY "Debug on"
           END-IF.
       SETUP.
           IF WS-VALUE > 0
               IF WS-MODE = 2
                   DISPLAY "Active"
               END-IF
           END-IF.
           MOVE 0 TO WS-VALUE.
