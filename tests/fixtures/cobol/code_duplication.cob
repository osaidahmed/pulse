       IDENTIFICATION DIVISION.
       PROGRAM-ID. DUP-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-A PIC 9(5).
       01 WS-B PIC 9(5).
       01 WS-C PIC 9(5).
       01 WS-TOTAL PIC 9(5).
       PROCEDURE DIVISION.
       CALC-REPORT-A.
           MOVE 0 TO WS-TOTAL.
           ADD WS-A TO WS-TOTAL.
           ADD WS-B TO WS-TOTAL.
           ADD WS-C TO WS-TOTAL.
           COMPUTE WS-TOTAL = WS-TOTAL * 2.
           DISPLAY WS-TOTAL.
           DISPLAY "Done A".
       CALC-REPORT-B.
           MOVE 0 TO WS-TOTAL.
           ADD WS-A TO WS-TOTAL.
           ADD WS-B TO WS-TOTAL.
           ADD WS-C TO WS-TOTAL.
           COMPUTE WS-TOTAL = WS-TOTAL * 2.
           DISPLAY WS-TOTAL.
           DISPLAY "Done B".
       CALC-REPORT-C.
           MOVE 0 TO WS-TOTAL.
           ADD WS-A TO WS-TOTAL.
           ADD WS-B TO WS-TOTAL.
           ADD WS-C TO WS-TOTAL.
           COMPUTE WS-TOTAL = WS-TOTAL * 2.
           DISPLAY WS-TOTAL.
           DISPLAY "Done C".
