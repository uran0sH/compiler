; ModuleID = 'module'
source_filename = "module"

@count = global i32 0
@n = global i32 3

define void @hanoi(i32 %0, i32 %1, i32 %2, i32 %3) {
hanoiEntry:
  %n = alloca i32, align 4
  store i32 %0, i32* %n, align 4
  %source = alloca i32, align 4
  store i32 %1, i32* %source, align 4
  %target = alloca i32, align 4
  store i32 %2, i32* %target, align 4
  %auxiliary = alloca i32, align 4
  store i32 %3, i32* %auxiliary, align 4
  %n1 = load i32, i32* %n, align 4
  %eqtmp = icmp eq i32 %n1, 1
  %eqext = zext i1 %eqtmp to i32
  %cond_2 = icmp ne i32 %eqext, 0
  br i1 %cond_2, label %if_true_, label %if_false_

if_true_:                                         ; preds = %hanoiEntry
  %count = load i32, i32* @count, align 4
  %tmp_ = add i32 %count, 1
  store i32 %tmp_, i32* @count, align 4
  br label %if_next_

if_false_:                                        ; preds = %hanoiEntry
  br label %if_next_

if_next_:                                         ; preds = %if_false_, %if_true_
  %n2 = load i32, i32* %n, align 4
  %tmp_3 = sub i32 %n2, 1
  %source4 = load i32, i32* %source, align 4
  %auxiliary5 = load i32, i32* %auxiliary, align 4
  %target6 = load i32, i32* %target, align 4
  call void @hanoi(i32 %tmp_3, i32 %source4, i32 %auxiliary5, i32 %target6)
  %count7 = load i32, i32* @count, align 4
  %tmp_8 = add i32 %count7, 1
  store i32 %tmp_8, i32* @count, align 4
  %n9 = load i32, i32* %n, align 4
  %tmp_10 = sub i32 %n9, 1
  %auxiliary11 = load i32, i32* %auxiliary, align 4
  %target12 = load i32, i32* %target, align 4
  %source13 = load i32, i32* %source, align 4
  call void @hanoi(i32 %tmp_10, i32 %auxiliary11, i32 %target12, i32 %source13)
}

define i32 @main() {
mainEntry:
  %n = load i32, i32* %n, align 4
  call void @hanoi(i32 %n, i32 1, i32 3, i32 2)
  %count = load i32, i32* @count, align 4
  ret i32 %count
}
