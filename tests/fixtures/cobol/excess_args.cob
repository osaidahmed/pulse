       IDENTIFICATION DIVISION.
       PROGRAM-ID. ARGS-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-A PIC X(10).
       01 WS-B PIC X(10).
       PROCEDURE DIVISION.
       CREATE-USER.
           MOVE "John" TO WS-A.
           MOVE "Doe" TO WS-B.
       SIMPLE-FUNC.
           DISPLAY WS-A.
           DISPLAY WS-B.
