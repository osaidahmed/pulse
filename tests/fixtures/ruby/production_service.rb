class OrderService
  def initialize(db, cache, logger, mailer, validator, config)
    @db = db
    @cache = cache
    @logger = logger
    @mailer = mailer
    @validator = validator
    @config = config
  end

  def process_order(order)
    return nil unless order

    if order[:status] == "pending"
      if order[:amount] > 1000
        if order[:verified]
          approve_large_order(order)
        else
          flag_for_review(order)
        end
      elsif order[:amount] > 100
        approve_standard_order(order)
      else
        approve_small_order(order)
      end
    elsif order[:status] == "review"
      if order[:reviewer_approved] && order[:manager_approved]
        finalize_order(order)
      elsif order[:reviewer_approved] || order[:escalated]
        escalate_order(order)
      end
    elsif order[:status] == "cancelled"
      refund_order(order)
    end
  end

  def validate_order(order)
    return false unless order[:id]
    return false unless order[:amount]
    return false if order[:amount] <= 0

    case order[:type]
    when "standard"
      order[:amount] < 10000
    when "premium"
      order[:amount] < 50000
    when "enterprise"
      true
    else
      false
    end
  end

  def generate_report(orders)
    result = 0
    orders.each do |o|
      if o[:status] == "completed"
        result += o[:amount]
      end
    end
    result
  end

  private

  def approve_large_order(order)
    @logger.info("Large order approved")
    @db.save(order)
  end

  def approve_standard_order(order)
    @db.save(order)
  end

  def approve_small_order(order)
    @db.save(order)
  end

  def flag_for_review(order)
    @logger.warn("Order flagged")
    @db.update(order, status: "review")
  end

  def finalize_order(order)
    @db.update(order, status: "completed")
    @mailer.send_confirmation(order)
  end

  def escalate_order(order)
    @logger.warn("Order escalated")
    @db.update(order, status: "escalated")
  end

  def refund_order(order)
    @db.update(order, status: "refunded")
    @mailer.send_refund_notice(order)
  end
end
