       IDENTIFICATION DIVISION.
       PROGRAM-ID. BUMPY-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-A PIC 9.
       01 WS-B PIC 9.
       01 WS-C PIC 9.
       01 WS-D PIC 9.
       01 WS-X PIC 9.
       PROCEDURE DIVISION.
       VALIDATE-DATA.
           IF WS-A > 0
               IF WS-B > 0
                   IF WS-C > 0
                       MOVE 1 TO WS-X
                   END-IF
               END-IF
           END-IF.
           MOVE 0 TO WS-X.
           IF WS-D > 0
               IF WS-A > 0
                   IF WS-B > 0
                       MOVE 2 TO WS-X
                   END-IF
               END-IF
           END-IF.
