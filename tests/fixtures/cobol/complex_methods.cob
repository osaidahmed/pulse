       IDENTIFICATION DIVISION.
       PROGRAM-ID. COMPLEX-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-STATUS PIC 9.
       01 WS-PRIORITY PIC 9.
       01 WS-AMOUNT PIC 9(5).
       01 WS-DISCOUNT PIC 9(3).
       01 WS-TAX PIC 9(3).
       01 WS-RESULT PIC 9(5).
       PROCEDURE DIVISION.
       PROCESS-ORDER.
           IF WS-STATUS = 1
               IF WS-PRIORITY > 5
                   COMPUTE WS-RESULT = WS-AMOUNT * 2
               ELSE
                   IF WS-PRIORITY > 3
                       MOVE WS-AMOUNT TO WS-RESULT
                   ELSE
                       COMPUTE WS-RESULT = WS-AMOUNT / 2
                   END-IF
               END-IF
           ELSE
               IF WS-STATUS = 2
                   IF WS-DISCOUNT > 0
                       COMPUTE WS-RESULT = WS-AMOUNT - WS-DISCOUNT
                   END-IF
               ELSE
                   IF WS-STATUS = 3
                       IF WS-PRIORITY = 1
                           COMPUTE WS-RESULT = WS-AMOUNT + WS-TAX
                       ELSE
                           IF WS-PRIORITY = 2
                               COMPUTE WS-RESULT = WS-AMOUNT - WS-TAX
                           ELSE
                               IF WS-PRIORITY = 3
                                   MOVE WS-AMOUNT TO WS-RESULT
                               ELSE
                                   MOVE 0 TO WS-RESULT
                               END-IF
                           END-IF
                       END-IF
                   END-IF
               END-IF
           END-IF.
           IF WS-RESULT < 0
               MOVE 0 TO WS-RESULT
           END-IF.
       SIMPLE-HELPER.
           ADD 1 TO WS-RESULT.
