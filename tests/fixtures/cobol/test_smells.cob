       IDENTIFICATION DIVISION.
       PROGRAM-ID. TEST-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-EXPECTED PIC 9(5).
       01 WS-ACTUAL PIC 9(5).
       PROCEDURE DIVISION.
       TEST-VALIDATE-INPUT.
           MOVE 100 TO WS-EXPECTED.
           MOVE 100 TO WS-ACTUAL.
           IF WS-EXPECTED NOT = WS-ACTUAL
               DISPLAY "FAIL: values differ"
           END-IF.
       TEST-BOUNDARY-CHECK.
           MOVE 0 TO WS-EXPECTED.
           MOVE 0 TO WS-ACTUAL.
           IF WS-EXPECTED NOT = WS-ACTUAL
               DISPLAY "FAIL: boundary"
           END-IF.
