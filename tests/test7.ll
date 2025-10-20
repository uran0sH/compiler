; ModuleID = 'module'
source_filename = "module"

@a = global i32 0
@count = global i32 0

define i32 @main() {
mainEntry:
  br label %whileCond

whileCond:                                        ; preds = %if_next_, %mainEntry
  %a = load i32, i32* @a, align 4
  %cond_ = icmp sle i32 %a, 0
  %cond_1 = zext i1 %cond_ to i32
  %cond_2 = icmp ne i32 %cond_1, 0
  br i1 %cond_2, label %whileBody, label %whileNext

whileBody:                                        ; preds = %whileCond
  %a1 = load i32, i32* @a, align 4
  %tmp_ = sub i32 %a1, 1
  store i32 %tmp_, i32* @a, align 4
  %count = load i32, i32* @count, align 4
  %tmp_2 = add i32 %count, 1
  store i32 %tmp_2, i32* @count, align 4
  %a3 = load i32, i32* @a, align 4
  %cond_4 = icmp slt i32 %a3, -20
  %cond_15 = zext i1 %cond_4 to i32
  %cond_26 = icmp ne i32 %cond_15, 0
  br i1 %cond_26, label %if_true_, label %if_false_

whileNext:                                        ; preds = %if_true_, %whileCond
  %count7 = load i32, i32* @count, align 4
  ret i32 %count7

if_true_:                                         ; preds = %whileBody
  br label %whileNext

if_false_:                                        ; preds = %whileBody
  br label %if_next_

if_next_:                                         ; preds = %if_false_
  br label %whileCond
}
