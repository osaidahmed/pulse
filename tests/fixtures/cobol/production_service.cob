      *> A realistic COBOL production service with multiple smell types.
       IDENTIFICATION DIVISION.
       PROGRAM-ID. ORDER-SERVICE.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-ORDER-ID PIC 9(8).
       01 WS-CUSTOMER-ID PIC 9(8).
       01 WS-STATUS PIC X(15).
       01 WS-PAYMENT-OK PIC 9.
       01 WS-SHIP-READY PIC 9.
       01 WS-CANCELLED PIC 9.
       01 WS-DELIVERED PIC 9.
       01 WS-ITEM-QTY PIC 9(3).
       01 WS-AVAIL PIC 9(3).
       01 WS-TOTAL PIC 9(7)V99.
       01 WS-DISCOUNT PIC 9(5)V99.
       01 WS-TAX PIC 9(5)V99.
       01 WS-TRACKING PIC X(20).
       01 WS-RESULT PIC X(20).
       01 WS-RETRY PIC 9.
       01 WS-I PIC 9(3).
       01 WS-LINE-AMT PIC 9(7)V99.
       01 WS-LINE-QTY PIC 9(3).
       01 WS-LINE-PRICE PIC 9(5)V99.
       01 WS-REPORT-A PIC X(80).
       01 WS-REPORT-B PIC X(80).
       PROCEDURE DIVISION.
       PROCESS-ORDER.
           IF WS-STATUS = "PENDING"
               IF WS-PAYMENT-OK = 1
                   IF WS-ITEM-QTY > 0
                       MOVE "PROCESSING" TO WS-STATUS
                       PERFORM VARYING WS-I FROM 1 BY 1
                           UNTIL WS-I > WS-ITEM-QTY
                           IF WS-ITEM-QTY > WS-AVAIL
                               MOVE "BACKORDER" TO WS-STATUS
                           END-IF
                       END-PERFORM
                   ELSE
                       MOVE "BACKORDERED" TO WS-STATUS
                   END-IF
               ELSE
                   PERFORM CHARGE-PAYMENT
                   IF WS-RETRY = 1
                       MOVE "PROCESSING" TO WS-STATUS
                   ELSE
                       IF WS-RETRY = 2
                           MOVE "PAY-PENDING" TO WS-STATUS
                       ELSE
                           MOVE "PAY-FAILED" TO WS-STATUS
                       END-IF
                   END-IF
               END-IF
           ELSE
               IF WS-STATUS = "PROCESSING"
                   IF WS-SHIP-READY = 1
                       PERFORM SHIP-ORDER
                       MOVE "SHIPPED" TO WS-STATUS
                   ELSE
                       IF WS-CANCELLED = 1
                           PERFORM RELEASE-INV
                           MOVE "CANCELLED" TO WS-STATUS
                       END-IF
                   END-IF
               ELSE
                   IF WS-STATUS = "SHIPPED"
                       IF WS-DELIVERED = 1
                           MOVE "DELIVERED" TO WS-STATUS
                       END-IF
                   END-IF
               END-IF
           END-IF.
           IF WS-DISCOUNT > 0 AND WS-TAX > 0
               IF WS-TOTAL > 1000
                   IF WS-ITEM-QTY > 5
                       COMPUTE WS-TOTAL = WS-TOTAL - WS-DISCOUNT
                   END-IF
               END-IF
           END-IF.
       GET-ORDER.
           DISPLAY WS-ORDER-ID.
           IF WS-ORDER-ID > 0
               DISPLAY WS-STATUS
           END-IF.
       SEND-CONFIRM.
           DISPLAY "Confirmed".
       PUBLISH-EVENT.
           DISPLAY "Event published".
       CHECK-RATE-LIMIT.
           DISPLAY "Rate OK".
       LOG-AUDIT.
           DISPLAY "Audit logged".
       CHARGE-PAYMENT.
           MOVE 1 TO WS-RETRY.
       SHIP-ORDER.
           MOVE "TRK-001" TO WS-TRACKING.
       RELEASE-INV.
           DISPLAY "Inventory released".
       GENERATE-INVOICE.
           MOVE 0 TO WS-TOTAL.
           PERFORM VARYING WS-I FROM 1 BY 1
               UNTIL WS-I > WS-ITEM-QTY
               COMPUTE WS-LINE-AMT = WS-LINE-QTY * WS-LINE-PRICE
               ADD WS-LINE-AMT TO WS-TOTAL
           END-PERFORM.
           DISPLAY WS-TOTAL.
       FORMAT-USER-REPORT.
           MOVE SPACES TO WS-REPORT-A.
           STRING WS-CUSTOMER-ID DELIMITED SIZE
                  " - " DELIMITED SIZE
                  WS-STATUS DELIMITED SIZE
                  INTO WS-REPORT-A
           END-STRING.
           DISPLAY WS-REPORT-A.
       FORMAT-ADMIN-REPORT.
           MOVE SPACES TO WS-REPORT-B.
           STRING WS-CUSTOMER-ID DELIMITED SIZE
                  " - " DELIMITED SIZE
                  WS-STATUS DELIMITED SIZE
                  INTO WS-REPORT-B
           END-STRING.
           DISPLAY WS-REPORT-B.
