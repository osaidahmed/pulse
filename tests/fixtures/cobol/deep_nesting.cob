       IDENTIFICATION DIVISION.
       PROGRAM-ID. NESTING-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-A PIC 9.
       01 WS-B PIC 9.
       01 WS-C PIC 9.
       01 WS-D PIC 9.
       01 WS-E PIC 9.
       01 WS-F PIC 9.
       01 WS-RESULT PIC 9.
       PROCEDURE DIVISION.
       DEEPLY-NESTED.
           IF WS-A > 0
               IF WS-B > 0
                   IF WS-C > 0
                       IF WS-D > 0
                           IF WS-E > 0
                               IF WS-F > 0
                                   MOVE 1 TO WS-RESULT
                               END-IF
                           END-IF
                       END-IF
                   END-IF
               END-IF
           END-IF.
       MODERATELY-NESTED.
           IF WS-A > 0
               IF WS-B > 0
                   MOVE 1 TO WS-RESULT
               END-IF
           END-IF.
